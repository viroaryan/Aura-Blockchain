package org.auranetwork.mesh

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.SocketTimeoutException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

class AuraVpnService : VpnService() {

    private var vpnInterface: ParcelFileDescriptor? = null
    private val isRunning = AtomicBoolean(false)
    private var serviceJob: Job? = null
    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    companion object {
        private const val TAG = "AuraVpnService"
        const val ACTION_CONNECT = "org.auranetwork.mesh.CONNECT"
        const val ACTION_DISCONNECT = "org.auranetwork.mesh.DISCONNECT"
        const val ACTION_STATE_CHANGED = "org.auranetwork.mesh.STATE_CHANGED"
        
        const val EXTRA_HOST_IP = "EXTRA_HOST_IP"
        const val EXTRA_HOST_PORT = "EXTRA_HOST_PORT"
        const val EXTRA_PAIR_CODE = "EXTRA_PAIR_CODE"
        const val EXTRA_IS_CONNECTED = "EXTRA_IS_CONNECTED"
        const val EXTRA_ERROR = "EXTRA_ERROR"

        val bytesTransferred = AtomicLong(0)
        val isConnectedState = AtomicBoolean(false)
        var lastErrorMessage: String? = null
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        Log.d(TAG, "onStartCommand action: $action")

        if (action == ACTION_DISCONNECT) {
            stopVpn()
            return START_NOT_STICKY
        }

        if (action == ACTION_CONNECT) {
            val hostIp = intent.getStringExtra(EXTRA_HOST_IP) ?: "103.21.244.18"
            val hostPort = intent.getIntExtra(EXTRA_HOST_PORT, 9000)
            val pairCode = intent.getStringExtra(EXTRA_PAIR_CODE) ?: "849201"

            startForegroundNotification(pairCode)
            startVpnTunnel(hostIp, hostPort, pairCode)
        }

        return START_STICKY
    }

    private fun startVpnTunnel(hostIp: String, hostPort: Int, pairCode: String) {
        if (isRunning.get()) return

        serviceJob?.cancel()
        serviceJob = serviceScope.launch {
            try {
                Log.d(TAG, "Configuring VPN Interface...")
                val builder = Builder()
                    .setSession("Aura Mesh 5G Tunnel")
                    .addAddress("10.8.0.2", 24)
                    .addDnsServer("1.1.1.1")
                    .addDnsServer("8.8.8.8")
                    .addRoute("0.0.0.0", 0) // Route all IPv4 traffic
                    .setMtu(1400)
                    .setBlocking(false)

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    builder.setMetered(false)
                }

                vpnInterface = builder.establish()
                if (vpnInterface == null) {
                    throw IllegalStateException("VPN establish returned null. Ensure no other VPN is running.")
                }

                isRunning.set(true)
                isConnectedState.set(true)
                lastErrorMessage = null
                broadcastState(true)

                val pfd = vpnInterface!!
                val inputStream = FileInputStream(pfd.fileDescriptor)
                val outputStream = FileOutputStream(pfd.fileDescriptor)

                val udpSocket = DatagramSocket()
                udpSocket.soTimeout = 2000
                protect(udpSocket) // CRITICAL: Prevent routing loop

                val hostAddress = withContext(Dispatchers.IO) {
                    try {
                        InetAddress.getByName(hostIp)
                    } catch (e: Exception) {
                        InetAddress.getByName("1.1.1.1") // fallback safe
                    }
                }

                val sessionKey = CryptoEngine.deriveSessionKey(
                    pairCode.toByteArray(Charsets.UTF_8),
                    "AURA_SECURE_SALT".toByteArray(Charsets.UTF_8)
                )

                val outBuffer = ByteArray(32768)
                var seq = 0L

                // Outbound Worker (App -> TUN -> Encrypt -> UDP Socket)
                val outboundJob = launch {
                    try {
                        while (isActive && isRunning.get()) {
                            val readBytes = inputStream.read(outBuffer)
                            if (readBytes > 0) {
                                val packetBytes = outBuffer.copyOf(readBytes)
                                val encryptedFrame = CryptoEngine.encryptPacket(packetBytes, sessionKey, seq++)
                                val packet = DatagramPacket(encryptedFrame, encryptedFrame.size, hostAddress, hostPort)
                                udpSocket.send(packet)
                                bytesTransferred.addAndGet(readBytes.toLong())
                            } else {
                                delay(2)
                            }
                        }
                    } catch (e: Exception) {
                        if (isRunning.get()) Log.w(TAG, "Outbound loop exception: ${e.message}")
                    }
                }

                // Inbound Worker (UDP Socket -> Decrypt -> TUN -> App)
                val inboundJob = launch {
                    val inBuffer = ByteArray(32768)
                    while (isActive && isRunning.get()) {
                        try {
                            val inPacket = DatagramPacket(inBuffer, inBuffer.size)
                            udpSocket.receive(inPacket)
                            if (inPacket.length >= 24) {
                                val frameBytes = inBuffer.copyOf(inPacket.length)
                                val decryptedPlaintext = CryptoEngine.decryptPacket(frameBytes, sessionKey)
                                outputStream.write(decryptedPlaintext)
                                bytesTransferred.addAndGet(decryptedPlaintext.size.toLong())
                            }
                        } catch (e: SocketTimeoutException) {
                            // Normal socket timeout, continue polling
                        } catch (e: Exception) {
                            if (isRunning.get()) Log.w(TAG, "Inbound loop exception: ${e.message}")
                        }
                    }
                }

                // Keep-Alive Ping Loop
                val pingJob = launch {
                    val pingBytes = "AURA_PING".toByteArray(Charsets.UTF_8)
                    while (isActive && isRunning.get()) {
                        delay(3000)
                        try {
                            val encPing = CryptoEngine.encryptPacket(pingBytes, sessionKey, seq++)
                            val pingPacket = DatagramPacket(encPing, encPing.size, hostAddress, hostPort)
                            udpSocket.send(pingPacket)
                        } catch (e: Exception) {
                            // ignore ping transient errors
                        }
                    }
                }

                outboundJob.join()
                inboundJob.join()
                pingJob.join()

            } catch (e: Exception) {
                Log.e(TAG, "VPN Tunnel Critical Failure", e)
                lastErrorMessage = e.message ?: "Failed to establish VPN tunnel"
                broadcastState(false, lastErrorMessage)
                stopVpn()
            }
        }
    }

    private fun startForegroundNotification(pairCode: String) {
        val channelId = "aura_mesh_vpn_channel"
        val notificationManager = getSystemService(NotificationManager::class.java)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Aura Mesh 5G Tunnel Service",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows live status and bandwidth telemetry of active 5G Mesh Tunnel"
                setShowBadge(false)
            }
            notificationManager.createNotificationChannel(channel)
        }

        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val notification: Notification = NotificationCompat.Builder(this, channelId)
            .setContentTitle("Aura 5G Tunnel Connected")
            .setContentText("100% of phone traffic routed through 5G Node #$pairCode")
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) { // Android 14 (API 34)
            startForeground(
                1001,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC or ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(1001, notification)
        }
    }

    private fun broadcastState(connected: Boolean, error: String? = null) {
        val intent = Intent(ACTION_STATE_CHANGED).apply {
            putExtra(EXTRA_IS_CONNECTED, connected)
            if (error != null) putExtra(EXTRA_ERROR, error)
            setPackage(packageName)
        }
        sendBroadcast(intent)
    }

    private fun stopVpn() {
        Log.d(TAG, "Stopping VPN Service...")
        isRunning.set(false)
        isConnectedState.set(false)
        serviceJob?.cancel()

        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing VPN interface", e)
        }
        vpnInterface = null

        broadcastState(false)
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        super.onDestroy()
        stopVpn()
    }
}
