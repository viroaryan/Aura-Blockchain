package org.auranetwork.mesh

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

class AuraVpnService : VpnService() {

    private var vpnInterface: ParcelFileDescriptor? = null
    private val isRunning = AtomicBoolean(false)
    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    companion object {
        const val ACTION_CONNECT = "org.auranetwork.mesh.CONNECT"
        const val ACTION_DISCONNECT = "org.auranetwork.mesh.DISCONNECT"
        const val EXTRA_HOST_IP = "EXTRA_HOST_IP"
        const val EXTRA_HOST_PORT = "EXTRA_HOST_PORT"
        const val EXTRA_PAIR_CODE = "EXTRA_PAIR_CODE"

        val bytesTransferred = AtomicLong(0)
        val isConnectedState = AtomicBoolean(false)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        if (action == ACTION_DISCONNECT) {
            stopVpn()
            return START_NOT_STICKY
        }

        if (action == ACTION_CONNECT && !isRunning.get()) {
            val hostIp = intent.getStringExtra(EXTRA_HOST_IP) ?: "103.21.244.18"
            val hostPort = intent.getIntExtra(EXTRA_HOST_PORT, 9000)
            val pairCode = intent.getStringExtra(EXTRA_PAIR_CODE) ?: "849201"

            startForegroundNotification(pairCode)
            startVpnTunnel(hostIp, hostPort)
        }

        return START_STICKY
    }

    private fun startVpnTunnel(hostIp: String, hostPort: Int) {
        try {
            val builder = Builder()
                .setSession("Aura Mesh Tunnel")
                .addAddress("10.8.0.2", 24)
                .addDnsServer("1.1.1.1")
                .addRoute("0.0.0.0", 0) // Route 100% of phone traffic!
                .setMtu(1500)

            vpnInterface = builder.establish()
            isRunning.set(true)
            isConnectedState.set(true)

            val pfd = vpnInterface ?: return
            val inputStream = FileInputStream(pfd.fileDescriptor)
            val outputStream = FileOutputStream(pfd.fileDescriptor)

            serviceScope.launch {
                val udpSocket = DatagramSocket()
                protect(udpSocket) // Prevent VPN from looping its own socket!

                val hostAddress = InetAddress.getByName(hostIp)
                val buffer = ByteArray(32768)
                val sessionKey = ByteArray(32) { 0x42 }
                var seq = 0L

                // Outbound loop: Phone Apps -> TUN -> Encrypt -> Host 5G
                launch {
                    try {
                        while (isRunning.get()) {
                            val length = inputStream.read(buffer)
                            if (length > 0) {
                                val packetBytes = buffer.copyOf(length)
                                val encryptedFrame = CryptoEngine.encryptPacket(packetBytes, sessionKey, seq++)
                                val packet = DatagramPacket(encryptedFrame, encryptedFrame.size, hostAddress, hostPort)
                                udpSocket.send(packet)
                                bytesTransferred.addAndGet(length.toLong())
                            }
                        }
                    } catch (e: Exception) {
                        e.printStackTrace()
                    }
                }

                // Inbound loop: Host 5G -> Decrypt -> TUN -> Phone Apps
                launch {
                    val inBuffer = ByteArray(32768)
                    try {
                        while (isRunning.get()) {
                            val inPacket = DatagramPacket(inBuffer, inBuffer.size)
                            udpSocket.receive(inPacket)
                            val frameBytes = inBuffer.copyOf(inPacket.length)
                            val decryptedPlaintext = CryptoEngine.decryptPacket(frameBytes, sessionKey)
                            outputStream.write(decryptedPlaintext)
                            bytesTransferred.addAndGet(decryptedPlaintext.size.toLong())
                        }
                    } catch (e: Exception) {
                        e.printStackTrace()
                    }
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
            stopVpn()
        }
    }

    private fun startForegroundNotification(pairCode: String) {
        val channelId = "aura_mesh_vpn"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Aura Mesh VPN Tunnel",
                NotificationManager.IMPORTANCE_LOW
            )
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }

        val intent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val notification: Notification = NotificationCompat.Builder(this, channelId)
            .setContentTitle("Aura 5G Tunnel Active")
            .setContentText("100% of device traffic routed through Peer #$pairCode")
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()

        startForeground(1001, notification)
    }

    private fun stopVpn() {
        isRunning.set(false)
        isConnectedState.set(false)
        serviceScope.cancel()
        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            e.printStackTrace()
        }
        vpnInterface = null
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        super.onDestroy()
        stopVpn()
    }
}
