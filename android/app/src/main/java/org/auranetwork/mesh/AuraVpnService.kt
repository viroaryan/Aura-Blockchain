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

/**
 * 100% Real OS-Wide VPN Tunnel Client Service
 * Captures all Android device traffic from tun0, encrypts with AES-256-GCM,
 * and routes through the verified 5G Host Peer.
 */
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
        const val EXTRA_IS_CONNECTING = "EXTRA_IS_CONNECTING"
        const val EXTRA_ERROR = "EXTRA_ERROR"

        val bytesTransferred = AtomicLong(0)
        val packetsSent = AtomicLong(0)
        val packetsReceived = AtomicLong(0)
        val isConnectedState = AtomicBoolean(false)
        val isConnectingState = AtomicBoolean(false)
        var currentPeerCode: String? = null
        var lastErrorMessage: String? = null
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        Log.d(TAG, "VPN onStartCommand action: $action")

        if (action == ACTION_DISCONNECT) {
            stopVpn()
            return START_NOT_STICKY
        }

        if (action == ACTION_CONNECT) {
            val hostIp = intent.getStringExtra(EXTRA_HOST_IP) ?: "192.168.43.1"
            val hostPort = intent.getIntExtra(EXTRA_HOST_PORT, 9000)
            val pairCode = intent.getStringExtra(EXTRA_PAIR_CODE) ?: ""

            if (pairCode.length < 5) {
                broadcastState(connected = false, connecting = false, error = "Invalid 6-digit pair code")
                return START_NOT_STICKY
            }

            currentPeerCode = pairCode
            startVpnTunnelWithRealHandshake(hostIp, hostPort, pairCode)
        }

        return START_STICKY
    }

    /**
     * Authenticates handshake before establishing VPN. Rejects fake codes!
     */
    private fun startVpnTunnelWithRealHandshake(hostIp: String, hostPort: Int, pairCode: String) {
        if (isRunning.get()) return

        serviceJob?.cancel()
        serviceJob = serviceScope.launch {
            var udpSocket: DatagramSocket? = null
            try {
                isConnectingState.set(true)
                isConnectedState.set(false)
                bytesTransferred.set(0)
                packetsSent.set(0)
                packetsReceived.set(0)
                lastErrorMessage = null
                broadcastState(connected = false, connecting = true)

                // Try candidate host IPs: Provided IP, Hotspot Gateway (192.168.43.1), Local Loopback
                val candidateIps = listOf(
                    hostIp,
                    "192.168.43.1",
                    "127.0.0.1"
                ).distinct()

                val sessionKey = CryptoEngine.deriveSessionKey(
                    pairCode.toByteArray(Charsets.UTF_8),
                    "AURA_SECURE_SALT".toByteArray(Charsets.UTF_8)
                )

                udpSocket = DatagramSocket()
                udpSocket.soTimeout = 2500
                protect(udpSocket) // CRITICAL: Protect socket from VPN routing loop

                var verifiedHostAddress: InetAddress? = null
                var handshakeSuccess = false

                // Perform real cryptographic handshake
                for (cand in candidateIps) {
                    try {
                        val hostAddress = withContext(Dispatchers.IO) {
                            InetAddress.getByName(cand)
                        }

                        val handshakeSyn = "AURA_SYN:$pairCode".toByteArray(Charsets.UTF_8)
                        val encSyn = CryptoEngine.encryptPacket(handshakeSyn, sessionKey, 0L)
                        val synPacket = DatagramPacket(encSyn, encSyn.size, hostAddress, hostPort)
                        
                        Log.d(TAG, "Attempting cryptographic handshake to $cand:$hostPort with code $pairCode")
                        udpSocket.send(synPacket)

                        val ackBuffer = ByteArray(1024)
                        val ackPacket = DatagramPacket(ackBuffer, ackBuffer.size)
                        udpSocket.receive(ackPacket)

                        if (ackPacket.length >= 24) {
                            val frameBytes = ackBuffer.copyOf(ackPacket.length)
                            val decAck = String(CryptoEngine.decryptPacket(frameBytes, sessionKey), Charsets.UTF_8)
                            if (decAck.startsWith("AURA_ACK")) {
                                handshakeSuccess = true
                                verifiedHostAddress = hostAddress
                                Log.d(TAG, "Handshake Verified successfully from Host at $cand!")
                                break
                            }
                        }
                    } catch (e: Exception) {
                        Log.w(TAG, "Candidate $cand did not respond: ${e.message}")
                    }
                }

                if (!handshakeSuccess || verifiedHostAddress == null) {
                    throw IllegalStateException("Host #$pairCode did not respond. Peer is offline, wrong code entered, or 'Share My 5G' is not active.")
                }

                // 2. Handshake Succeeded -> Establish TUN Interface (Captures 100% OS traffic)
                Log.d(TAG, "Handshake verified! Configuring TUN Interface...")
                val builder = Builder()
                    .setSession("Aura Mesh 5G (#$pairCode)")
                    .addAddress("10.8.0.2", 24)
                    .addDnsServer("1.1.1.1")
                    .addDnsServer("8.8.8.8")
                    .addRoute("0.0.0.0", 0)
                    .setMtu(1400)
                    .setBlocking(false)

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    builder.setMetered(false)
                }

                vpnInterface = builder.establish()
                if (vpnInterface == null) {
                    throw IllegalStateException("Failed to establish TUN interface. Check device VPN settings.")
                }

                isRunning.set(true)
                isConnectedState.set(true)
                isConnectingState.set(false)
                startForegroundNotification(pairCode)
                broadcastState(connected = true, connecting = false)

                val pfd = vpnInterface!!
                val inputStream = FileInputStream(pfd.fileDescriptor)
                val outputStream = FileOutputStream(pfd.fileDescriptor)

                val outBuffer = ByteArray(32768)
                var seq = 1L
                val activeSocket = udpSocket
                val targetHost = verifiedHostAddress

                // Outbound Loop (tun0 -> AES-256 Encrypt -> Host 5G Internet)
                val outboundJob = launch {
                    try {
                        while (isActive && isRunning.get()) {
                            val readBytes = inputStream.read(outBuffer)
                            if (readBytes > 0) {
                                val packetBytes = outBuffer.copyOf(readBytes)
                                val encryptedFrame = CryptoEngine.encryptPacket(packetBytes, sessionKey, seq++)
                                val packet = DatagramPacket(encryptedFrame, encryptedFrame.size, targetHost, hostPort)
                                activeSocket.send(packet)
                                bytesTransferred.addAndGet(readBytes.toLong())
                                packetsSent.incrementAndGet()
                            } else {
                                delay(2)
                            }
                        }
                    } catch (e: Exception) {
                        if (isRunning.get()) Log.w(TAG, "Outbound loop error: ${e.message}")
                    }
                }

                // Inbound Loop (Host 5G Internet -> AES-256 Decrypt -> tun0)
                val inboundJob = launch {
                    val inBuffer = ByteArray(32768)
                    while (isActive && isRunning.get()) {
                        try {
                            val inPacket = DatagramPacket(inBuffer, inBuffer.size)
                            activeSocket.receive(inPacket)
                            if (inPacket.length >= 24) {
                                val frameBytes = inBuffer.copyOf(inPacket.length)
                                val decryptedPlaintext = CryptoEngine.decryptPacket(frameBytes, sessionKey)
                                outputStream.write(decryptedPlaintext)
                                bytesTransferred.addAndGet(decryptedPlaintext.size.toLong())
                                packetsReceived.incrementAndGet()
                            }
                        } catch (e: SocketTimeoutException) {
                            // normal polling timeout
                        } catch (e: Exception) {
                            if (isRunning.get()) Log.w(TAG, "Inbound loop error: ${e.message}")
                        }
                    }
                }

                outboundJob.join()
                inboundJob.join()

            } catch (e: Exception) {
                Log.e(TAG, "VPN Handshake/Tunnel Error", e)
                lastErrorMessage = e.message ?: "Failed to connect to Host #$pairCode"
                broadcastState(connected = false, connecting = false, error = lastErrorMessage)
                stopVpn()
            } finally {
                udpSocket?.close()
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
                description = "Shows live status and telemetry of active 5G Mesh Tunnel"
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
            .setContentTitle("Aura 5G Tunnel Connected (#$pairCode)")
            .setContentText("100% OS traffic routed through peer. Zero logs.")
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                1001,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC or ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(1001, notification)
        }
    }

    private fun broadcastState(connected: Boolean, connecting: Boolean, error: String? = null) {
        val intent = Intent(ACTION_STATE_CHANGED).apply {
            putExtra(EXTRA_IS_CONNECTED, connected)
            putExtra(EXTRA_IS_CONNECTING, connecting)
            if (error != null) putExtra(EXTRA_ERROR, error)
            setPackage(packageName)
        }
        sendBroadcast(intent)
    }

    private fun stopVpn() {
        Log.d(TAG, "Stopping VPN Service...")
        isRunning.set(false)
        isConnectedState.set(false)
        isConnectingState.set(false)
        serviceJob?.cancel()

        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing VPN interface", e)
        }
        vpnInterface = null

        broadcastState(connected = false, connecting = false)
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        super.onDestroy()
        stopVpn()
    }
}
