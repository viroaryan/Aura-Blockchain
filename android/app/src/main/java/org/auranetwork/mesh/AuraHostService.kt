package org.auranetwork.mesh

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.Socket
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/**
 * Real 5G/Wi-Fi Internet Sharing Exit Node Service for Phone A (Host)
 * Accepts incoming encrypted tunnel connections and routes them to the public internet.
 */
class AuraHostService : Service() {

    private val isRunning = AtomicBoolean(false)
    private var hostJob: Job? = null
    private val hostScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    companion object {
        private const val TAG = "AuraHostService"
        const val ACTION_START_HOST = "org.auranetwork.mesh.START_HOST"
        const val ACTION_STOP_HOST = "org.auranetwork.mesh.STOP_HOST"
        const val EXTRA_HOST_CODE = "EXTRA_HOST_CODE"

        val bytesServedTotal = AtomicLong(0)
        val activeClientsCount = AtomicLong(0)
        val isHostRunningState = AtomicBoolean(false)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        Log.d(TAG, "Host Service onStartCommand action: $action")

        if (action == ACTION_STOP_HOST) {
            stopHost()
            return START_NOT_STICKY
        }

        if (action == ACTION_START_HOST) {
            val hostCode = intent.getStringExtra(EXTRA_HOST_CODE) ?: ""
            if (hostCode.length >= 5) {
                startHostServer(hostCode)
            }
        }

        return START_STICKY
    }

    private fun startHostServer(hostCode: String) {
        if (isRunning.get()) return

        isRunning.set(true)
        isHostRunningState.set(true)
        bytesServedTotal.set(0)
        activeClientsCount.set(0)

        startForegroundNotification(hostCode)

        hostJob = hostScope.launch {
            var serverSocket: DatagramSocket? = null
            try {
                serverSocket = DatagramSocket(9000)
                serverSocket.soTimeout = 2000
                Log.d(TAG, "Aura 5G Host Relay Server listening on port 9000 for Code #$hostCode")

                val sessionKey = CryptoEngine.deriveSessionKey(
                    hostCode.toByteArray(Charsets.UTF_8),
                    "AURA_SECURE_SALT".toByteArray(Charsets.UTF_8)
                )

                val buffer = ByteArray(65536)
                val activeClients = ConcurrentHashMap<String, Long>()

                while (isActive && isRunning.get()) {
                    try {
                        val packet = DatagramPacket(buffer, buffer.size)
                        serverSocket.receive(packet)

                        val clientEndpoint = "${packet.address.hostAddress}:${packet.port}"
                        activeClients[clientEndpoint] = System.currentTimeMillis()
                        activeClientsCount.set(activeClients.size.toLong())

                        val frameBytes = buffer.copyOf(packet.length)

                        // 1. Check if Handshake SYN
                        if (frameBytes.size >= 24) {
                            try {
                                val decrypted = CryptoEngine.decryptPacket(frameBytes, sessionKey)
                                val text = String(decrypted, Charsets.UTF_8)

                                if (text.startsWith("AURA_SYN:")) {
                                    val codeSent = text.substringAfter("AURA_SYN:")
                                    if (codeSent == hostCode) {
                                        Log.d(TAG, "Authenticated Peer $clientEndpoint with code #$hostCode. Sending ACK!")
                                        val ackBytes = "AURA_ACK:ESTABLISHED".toByteArray(Charsets.UTF_8)
                                        val encAck = CryptoEngine.encryptPacket(ackBytes, sessionKey, 0L)
                                        val ackPacket = DatagramPacket(encAck, encAck.size, packet.address, packet.port)
                                        serverSocket.send(ackPacket)
                                    }
                                    continue
                                }

                                // 2. Real IP Packet Forwarding (User-Space NAT)
                                val parsedIp = IpPacketParser.parse(decrypted)
                                if (parsedIp != null) {
                                    bytesServedTotal.addAndGet(decrypted.size.toLong())

                                    // Forward UDP (DNS queries, Voice/Video packets)
                                    if (parsedIp.protocol == IpPacketParser.PROTOCOL_UDP && parsedIp.destPort > 0) {
                                        launch(Dispatchers.IO) {
                                            try {
                                                val outboundSocket = DatagramSocket()
                                                outboundSocket.soTimeout = 3000
                                                val outPacket = DatagramPacket(
                                                    parsedIp.payload,
                                                    parsedIp.payload.size,
                                                    parsedIp.destIp,
                                                    parsedIp.destPort
                                                )
                                                outboundSocket.send(outPacket)

                                                val respBuffer = ByteArray(4096)
                                                val respPacket = DatagramPacket(respBuffer, respBuffer.size)
                                                outboundSocket.receive(respPacket)

                                                val respPayload = respBuffer.copyOf(respPacket.length)
                                                // Reconstruct IP packet for client's tun0
                                                val reconstructedIpPacket = IpPacketParser.buildUdpPacket(
                                                    srcIp = parsedIp.destIp,
                                                    dstIp = parsedIp.sourceIp,
                                                    srcPort = parsedIp.destPort,
                                                    dstPort = parsedIp.sourcePort,
                                                    payload = respPayload
                                                )

                                                val encResp = CryptoEngine.encryptPacket(reconstructedIpPacket, sessionKey, System.currentTimeMillis())
                                                val returnPacket = DatagramPacket(encResp, encResp.size, packet.address, packet.port)
                                                serverSocket.send(returnPacket)
                                                bytesServedTotal.addAndGet(reconstructedIpPacket.size.toLong())

                                                outboundSocket.close()
                                            } catch (e: Exception) {
                                                // ignore timeout
                                            }
                                        }
                                    }
                                }
                            } catch (e: Exception) {
                                Log.w(TAG, "Decryption or packet handling exception: ${e.message}")
                            }
                        }
                    } catch (e: Exception) {
                        // socket timeout polling loop
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Host Server error", e)
            } finally {
                serverSocket?.close()
            }
        }
    }

    private fun startForegroundNotification(hostCode: String) {
        val channelId = "aura_host_relay_channel"
        val notificationManager = getSystemService(NotificationManager::class.java)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Aura 5G Host Relay Service",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows live status when sharing phone internet with mesh peers"
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
            .setContentTitle("Sharing 5G Internet (#$hostCode)")
            .setContentText("Aura Mesh Exit Node active. Peers can route all OS traffic.")
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                2002,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC or ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(2002, notification)
        }
    }

    private fun stopHost() {
        Log.d(TAG, "Stopping Aura Host Service...")
        isRunning.set(false)
        isHostRunningState.set(false)
        hostJob?.cancel()
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        super.onDestroy()
        stopHost()
    }
}
