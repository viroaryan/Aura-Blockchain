package org.auranetwork.mesh

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

data class PendingPairRequest(
    val clientIp: String,
    val clientPort: Int,
    val clientDeviceName: String,
    val otpCode: String,
    val timestamp: Long
)

/**
 * Real 5G/Wi-Fi Internet Sharing Exit Node Service for Phone A (Host)
 * Listens for LAN/Hotspot broadcast pair requests, handles 30s OTP validation,
 * prompts host user for approval, and routes encrypted TUN packets to public internet.
 */
class AuraHostService : Service() {

    private val isRunning = AtomicBoolean(false)
    private var hostJob: Job? = null
    private val hostScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var currentHostOtp: String = ""
    private var previousHostOtp: String = ""

    companion object {
        private const val TAG = "AuraHostService"
        const val ACTION_START_HOST = "org.auranetwork.mesh.START_HOST"
        const val ACTION_STOP_HOST = "org.auranetwork.mesh.STOP_HOST"
        const val ACTION_UPDATE_OTP = "org.auranetwork.mesh.UPDATE_OTP"
        const val ACTION_APPROVE_PEER = "org.auranetwork.mesh.APPROVE_PEER"
        const val ACTION_REJECT_PEER = "org.auranetwork.mesh.REJECT_PEER"
        const val ACTION_PAIR_REQUEST_ARRIVED = "org.auranetwork.mesh.PAIR_REQUEST_ARRIVED"

        const val EXTRA_HOST_OTP = "EXTRA_HOST_OTP"
        const val EXTRA_PREV_OTP = "EXTRA_PREV_OTP"
        const val EXTRA_CLIENT_IP = "EXTRA_CLIENT_IP"
        const val EXTRA_CLIENT_PORT = "EXTRA_CLIENT_PORT"
        const val EXTRA_CLIENT_NAME = "EXTRA_CLIENT_NAME"

        val bytesServedTotal = AtomicLong(0)
        val activeClientsCount = AtomicLong(0)
        val isHostRunningState = AtomicBoolean(false)
        var activePendingRequest: PendingPairRequest? = null
        val approvedClients = ConcurrentHashMap<String, Long>() // clientEndpoint -> approvedTimestamp
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        Log.d(TAG, "Host Service onStartCommand: $action")

        when (action) {
            ACTION_STOP_HOST -> {
                stopHost()
                return START_NOT_STICKY
            }
            ACTION_START_HOST -> {
                val otp = intent.getStringExtra(EXTRA_HOST_OTP) ?: ""
                if (otp.length >= 5) {
                    currentHostOtp = otp
                    previousHostOtp = intent.getStringExtra(EXTRA_PREV_OTP) ?: otp
                    startHostServer()
                }
            }
            ACTION_UPDATE_OTP -> {
                val newOtp = intent.getStringExtra(EXTRA_HOST_OTP) ?: ""
                if (newOtp.isNotEmpty()) {
                    previousHostOtp = currentHostOtp
                    currentHostOtp = newOtp
                    Log.d(TAG, "Host OTP Updated: $currentHostOtp (Prev: $previousHostOtp)")
                }
            }
            ACTION_APPROVE_PEER -> {
                val clientIp = intent.getStringExtra(EXTRA_CLIENT_IP) ?: ""
                val clientPort = intent.getIntExtra(EXTRA_CLIENT_PORT, 0)
                if (clientIp.isNotEmpty() && clientPort > 0) {
                    approvePeer(clientIp, clientPort)
                }
            }
            ACTION_REJECT_PEER -> {
                val clientIp = intent.getStringExtra(EXTRA_CLIENT_IP) ?: ""
                val clientPort = intent.getIntExtra(EXTRA_CLIENT_PORT, 0)
                if (clientIp.isNotEmpty() && clientPort > 0) {
                    rejectPeer(clientIp, clientPort)
                }
            }
        }

        return START_STICKY
    }

    private fun startHostServer() {
        if (isRunning.get()) return

        isRunning.set(true)
        isHostRunningState.set(true)
        bytesServedTotal.set(0)
        activeClientsCount.set(0)
        approvedClients.clear()
        activePendingRequest = null

        startForegroundNotification(currentHostOtp)

        hostJob = hostScope.launch {
            var serverSocket: DatagramSocket? = null
            try {
                // Bind to all network interfaces on port 9000
                serverSocket = DatagramSocket(9000, InetAddress.getByName("0.0.0.0"))
                serverSocket.broadcast = true
                serverSocket.soTimeout = 1500
                Log.d(TAG, "Aura 5G Host Relay listening on 0.0.0.0:9000 (OTP: $currentHostOtp)")

                val buffer = ByteArray(65536)

                while (isActive && isRunning.get()) {
                    try {
                        val packet = DatagramPacket(buffer, buffer.size)
                        serverSocket.receive(packet)

                        val clientIp = packet.address.hostAddress ?: ""
                        val clientPort = packet.port
                        val clientEndpoint = "$clientIp:$clientPort"
                        val packetLength = packet.length
                        val rawData = buffer.copyOf(packetLength)

                        // 1. Check if Plaintext PAIR REQUEST broadcast: AURA_PAIR_REQ:<otp>:<clientDeviceName>
                        val text = try { String(rawData, Charsets.UTF_8) } catch (e: Exception) { "" }

                        if (text.startsWith("AURA_PAIR_REQ:")) {
                            val parts = text.split(":")
                            if (parts.size >= 3) {
                                val reqOtp = parts[1].trim()
                                val clientDeviceName = parts[2].trim()

                                Log.d(TAG, "Received Pair Request from $clientEndpoint ($clientDeviceName) with OTP: $reqOtp")

                                // Validate against current or previous 30s OTP
                                if (reqOtp == currentHostOtp || reqOtp == previousHostOtp) {
                                    val req = PendingPairRequest(
                                        clientIp = clientIp,
                                        clientPort = clientPort,
                                        clientDeviceName = clientDeviceName,
                                        otpCode = reqOtp,
                                        timestamp = System.currentTimeMillis()
                                    )
                                    activePendingRequest = req

                                    // Broadcast to UI to show Approval Dialog
                                    val reqIntent = Intent(ACTION_PAIR_REQUEST_ARRIVED).apply {
                                        putExtra(EXTRA_CLIENT_IP, clientIp)
                                        putExtra(EXTRA_CLIENT_PORT, clientPort)
                                        putExtra(EXTRA_CLIENT_NAME, clientDeviceName)
                                        putExtra(EXTRA_HOST_OTP, reqOtp)
                                        setPackage(packageName)
                                    }
                                    sendBroadcast(reqIntent)

                                    // Show High-Priority Heads-Up Approval Notification
                                    showApprovalNotification(req)
                                } else {
                                    Log.w(TAG, "Rejected Pair Request: OTP mismatch ($reqOtp != $currentHostOtp)")
                                    val rejBytes = "AURA_PAIR_REJECTED:INVALID_CODE".toByteArray(Charsets.UTF_8)
                                    val rejPacket = DatagramPacket(rejBytes, rejBytes.size, packet.address, clientPort)
                                    serverSocket.send(rejPacket)
                                }
                            }
                            continue
                        }

                        // 2. Check if Peer is already Approved
                        if (!approvedClients.containsKey(clientEndpoint) && !approvedClients.containsKey(clientIp)) {
                            // Client not approved yet
                            continue
                        }

                        // Update active peer timestamp
                        approvedClients[clientEndpoint] = System.currentTimeMillis()
                        activeClientsCount.set(approvedClients.size.toLong())

                        // 3. Process Encrypted Packet using OTP Session Key
                        val effectiveOtp = if (currentHostOtp.isNotEmpty()) currentHostOtp else previousHostOtp
                        val sessionKey = CryptoEngine.deriveSessionKey(
                            effectiveOtp.toByteArray(Charsets.UTF_8),
                            "AURA_SECURE_SALT".toByteArray(Charsets.UTF_8)
                        )

                        if (rawData.size >= 24) {
                            try {
                                val decrypted = CryptoEngine.decryptPacket(rawData, sessionKey)
                                val decStr = String(decrypted, Charsets.UTF_8)

                                if (decStr.startsWith("AURA_SYN:")) {
                                    val ackBytes = "AURA_ACK:ESTABLISHED".toByteArray(Charsets.UTF_8)
                                    val encAck = CryptoEngine.encryptPacket(ackBytes, sessionKey, 0L)
                                    val ackPacket = DatagramPacket(encAck, encAck.size, packet.address, packet.port)
                                    serverSocket.send(ackPacket)
                                    continue
                                }

                                // User-Space NAT Packet Forwarding to Public Internet
                                val parsedIp = IpPacketParser.parse(decrypted)
                                if (parsedIp != null) {
                                    bytesServedTotal.addAndGet(decrypted.size.toLong())

                                    // Forward UDP (DNS queries, QUIC, Media Streams)
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
                                                val reconstructed = IpPacketParser.buildUdpPacket(
                                                    srcIp = parsedIp.destIp,
                                                    dstIp = parsedIp.sourceIp,
                                                    srcPort = parsedIp.destPort,
                                                    dstPort = parsedIp.sourcePort,
                                                    payload = respPayload
                                                )

                                                val encResp = CryptoEngine.encryptPacket(reconstructed, sessionKey, System.currentTimeMillis())
                                                val retPacket = DatagramPacket(encResp, encResp.size, packet.address, packet.port)
                                                serverSocket.send(retPacket)
                                                bytesServedTotal.addAndGet(reconstructed.size.toLong())
                                                outboundSocket.close()
                                            } catch (e: Exception) {
                                                // UDP forward timeout / unreachable
                                            }
                                        }
                                    }
                                }
                            } catch (e: Exception) {
                                // Decryption error
                            }
                        }
                    } catch (e: Exception) {
                        // socket timeout, continue polling
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Host Server error", e)
            } finally {
                serverSocket?.close()
            }
        }
    }

    private fun approvePeer(clientIp: String, clientPort: Int) {
        hostScope.launch {
            try {
                val endpoint = "$clientIp:$clientPort"
                approvedClients[endpoint] = System.currentTimeMillis()
                approvedClients[clientIp] = System.currentTimeMillis()
                activeClientsCount.set(approvedClients.size.toLong())
                activePendingRequest = null

                Log.d(TAG, "Host Approved Peer $endpoint! Sending AURA_PAIR_APPROVED")

                val sock = DatagramSocket()
                val approveMsg = "AURA_PAIR_APPROVED:$currentHostOtp:HOST_OK".toByteArray(Charsets.UTF_8)
                val targetAddr = InetAddress.getByName(clientIp)
                val packet = DatagramPacket(approveMsg, approveMsg.size, targetAddr, clientPort)
                sock.send(packet)
                sock.close()

                // Dismiss approval notification
                val notificationManager = getSystemService(NotificationManager::class.java)
                notificationManager.cancel(2002)
            } catch (e: Exception) {
                Log.e(TAG, "Error approving peer", e)
            }
        }
    }

    private fun rejectPeer(clientIp: String, clientPort: Int) {
        hostScope.launch {
            try {
                val endpoint = "$clientIp:$clientPort"
                approvedClients.remove(endpoint)
                approvedClients.remove(clientIp)
                activePendingRequest = null

                Log.d(TAG, "Host Rejected Peer $endpoint")

                val sock = DatagramSocket()
                val rejMsg = "AURA_PAIR_REJECTED:USER_DECLINED".toByteArray(Charsets.UTF_8)
                val targetAddr = InetAddress.getByName(clientIp)
                val packet = DatagramPacket(rejMsg, rejMsg.size, targetAddr, clientPort)
                sock.send(packet)
                sock.close()

                val notificationManager = getSystemService(NotificationManager::class.java)
                notificationManager.cancel(2002)
            } catch (e: Exception) {
                Log.e(TAG, "Error rejecting peer", e)
            }
        }
    }

    private fun showApprovalNotification(req: PendingPairRequest) {
        val channelId = "aura_approval_channel"
        val notificationManager = getSystemService(NotificationManager::class.java)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Aura 5G Connection Approval",
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Prompts to approve incoming 5G sharing requests"
                setShowBadge(true)
            }
            notificationManager.createNotificationChannel(channel)
        }

        val approveIntent = Intent(this, AuraHostService::class.java).apply {
            action = ACTION_APPROVE_PEER
            putExtra(EXTRA_CLIENT_IP, req.clientIp)
            putExtra(EXTRA_CLIENT_PORT, req.clientPort)
        }
        val approvePendingIntent = PendingIntent.getService(
            this, 1, approveIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val rejectIntent = Intent(this, AuraHostService::class.java).apply {
            action = ACTION_REJECT_PEER
            putExtra(EXTRA_CLIENT_IP, req.clientIp)
            putExtra(EXTRA_CLIENT_PORT, req.clientPort)
        }
        val rejectPendingIntent = PendingIntent.getService(
            this, 2, rejectIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val notification = NotificationCompat.Builder(this, channelId)
            .setContentTitle("⚡ 5G Connection Request")
            .setContentText("${req.clientDeviceName} (${req.clientIp}) wants to pair with OTP #${req.otpCode}")
            .setSmallIcon(R.mipmap.ic_launcher)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_CALL)
            .addAction(R.mipmap.ic_launcher, "Approve & Share", approvePendingIntent)
            .addAction(R.mipmap.ic_launcher, "Reject", rejectPendingIntent)
            .setAutoCancel(true)
            .build()

        notificationManager.notify(2002, notification)
    }

    private fun startForegroundNotification(otp: String) {
        val channelId = "aura_mesh_host_channel"
        val notificationManager = getSystemService(NotificationManager::class.java)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Aura 5G Hotspot Relay Active",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows live status of active 5G Hotspot Relay"
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
            .setContentTitle("Aura 5G Hotspot Server Active (OTP #$otp)")
            .setContentText("Listening for peer pair requests on port 9000")
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                1002,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC or ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(1002, notification)
        }
    }

    private fun stopHost() {
        Log.d(TAG, "Stopping Aura Host Service...")
        isRunning.set(false)
        isHostRunningState.set(false)
        hostJob?.cancel()
        approvedClients.clear()
        activePendingRequest = null

        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        super.onDestroy()
        stopHost()
    }
}
