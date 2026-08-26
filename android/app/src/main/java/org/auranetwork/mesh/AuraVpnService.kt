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
import java.net.NetworkInterface
import java.net.SocketTimeoutException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/**
 * 100% Real OS-Wide VPN Tunnel Client Service
 * Broadcasts pair requests across LAN & Hotspot, awaits Host's live consent approval,
 * captures 100% device OS traffic from tun0, encrypts with AES-256-GCM, and routes through the host.
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
        const val EXTRA_STATUS_MSG = "EXTRA_STATUS_MSG"
        const val EXTRA_ERROR = "EXTRA_ERROR"

        val bytesTransferred = AtomicLong(0)
        val packetsSent = AtomicLong(0)
        val packetsReceived = AtomicLong(0)
        val isConnectedState = AtomicBoolean(false)
        val isConnectingState = AtomicBoolean(false)
        var currentPeerCode: String? = null
        var currentStatusMessage: String? = null
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
            val customIp = intent.getStringExtra(EXTRA_HOST_IP) ?: ""
            val hostPort = intent.getIntExtra(EXTRA_HOST_PORT, 9000)
            val pairCode = intent.getStringExtra(EXTRA_PAIR_CODE) ?: ""

            if (pairCode.length < 5) {
                broadcastState(connected = false, connecting = false, error = "Please enter a valid 6-digit Host OTP code")
                return START_NOT_STICKY
            }

            currentPeerCode = pairCode
            startVpnTunnelWithLiveApproval(customIp, hostPort, pairCode)
        }

        return START_STICKY
    }

    private fun startVpnTunnelWithLiveApproval(customIp: String, hostPort: Int, pairCode: String) {
        if (isRunning.get()) return

        serviceJob?.cancel()
        serviceJob = serviceScope.launch {
            var clientSocket: DatagramSocket? = null
            try {
                isConnectingState.set(true)
                isConnectedState.set(false)
                bytesTransferred.set(0)
                packetsSent.set(0)
                packetsReceived.set(0)
                lastErrorMessage = null
                currentStatusMessage = "Broadcasting Pair Request with OTP #$pairCode..."
                broadcastState(connected = false, connecting = true, statusMsg = currentStatusMessage)

                // 1. Gather all local network broadcast & candidate IP targets
                val candidateBroadcastTargets = mutableListOf<String>()
                if (customIp.isNotEmpty()) candidateBroadcastTargets.add(customIp)
                candidateBroadcastTargets.add("192.168.43.1") // Android Tethering Hotspot default
                candidateBroadcastTargets.add("192.168.43.255")
                candidateBroadcastTargets.add("255.255.255.255")

                // Auto-detect local subnet broadcast addresses from network interfaces
                try {
                    val interfaces = NetworkInterface.getNetworkInterfaces()
                    while (interfaces.hasMoreElements()) {
                        val iface = interfaces.nextElement()
                        if (iface.isLoopback || !iface.isUp) continue
                        for (addr in iface.interfaceAddresses) {
                            val bcast = addr.broadcast
                            if (bcast != null && bcast.hostAddress != null) {
                                candidateBroadcastTargets.add(bcast.hostAddress!!)
                            }
                        }
                    }
                } catch (e: Exception) {
                    Log.w(TAG, "Failed discovering subnet broadcasts: ${e.message}")
                }

                val distinctTargets = candidateBroadcastTargets.distinct()
                Log.d(TAG, "Pair targets: $distinctTargets")

                clientSocket = DatagramSocket()
                clientSocket.broadcast = true
                clientSocket.soTimeout = 3000
                protect(clientSocket) // Protect from VPN tunnel loop

                val clientDeviceModel = "${Build.MANUFACTURER} ${Build.MODEL}".replace(":", " ")
                val pairReqPayload = "AURA_PAIR_REQ:$pairCode:$clientDeviceModel".toByteArray(Charsets.UTF_8)

                var verifiedHostAddress: InetAddress? = null
                var hostApproved = false
                var attempts = 0
                val maxAttempts = 6 // 6 attempts * 2.5s = 15s timeout

                currentStatusMessage = "Waiting for Host to Approve connection..."
                broadcastState(connected = false, connecting = true, statusMsg = currentStatusMessage)

                while (attempts < maxAttempts && !hostApproved && isActive) {
                    attempts++
                    // Broadcast pair request across all network candidates
                    for (target in distinctTargets) {
                        try {
                            val targetAddr = InetAddress.getByName(target)
                            val reqPacket = DatagramPacket(pairReqPayload, pairReqPayload.size, targetAddr, hostPort)
                            clientSocket.send(reqPacket)
                        } catch (e: Exception) {
                            // ignore individual route error
                        }
                    }

                    // Await response
                    val recvBuffer = ByteArray(2048)
                    val recvPacket = DatagramPacket(recvBuffer, recvBuffer.size)
                    try {
                        clientSocket.receive(recvPacket)
                        val respText = String(recvBuffer, 0, recvPacket.length, Charsets.UTF_8)
                        Log.d(TAG, "Received Host Response from ${recvPacket.address}: $respText")

                        if (respText.startsWith("AURA_PAIR_APPROVED:")) {
                            hostApproved = true
                            verifiedHostAddress = recvPacket.address
                            Log.d(TAG, "Pairing APPROVED by Host at ${recvPacket.address}!")
                            break
                        } else if (respText.startsWith("AURA_PAIR_REJECTED:")) {
                            throw IllegalStateException("Connection rejected by Host or invalid OTP code.")
                        }
                    } catch (e: SocketTimeoutException) {
                        // Retry broadcast
                    }
                }

                if (!hostApproved || verifiedHostAddress == null) {
                    throw IllegalStateException("Host did not respond or approve. Make sure Host has 'Share My 5G' enabled and is on the same Wi-Fi/Hotspot.")
                }

                // 2. Derive Session Encryption Key from Pair Code
                val sessionKey = CryptoEngine.deriveSessionKey(
                    pairCode.toByteArray(Charsets.UTF_8),
                    "AURA_SECURE_SALT".toByteArray(Charsets.UTF_8)
                )

                // 3. Establish TUN Interface (Interceps 100% OS traffic)
                Log.d(TAG, "Configuring TUN interface routing to ${verifiedHostAddress.hostAddress}...")
                val builder = Builder()
                    .setSession("Aura 5G Mesh (#$pairCode)")
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
                    throw IllegalStateException("Failed to establish TUN interface on Android device.")
                }

                isRunning.set(true)
                isConnectedState.set(true)
                isConnectingState.set(false)
                currentStatusMessage = "Connected to Host (${verifiedHostAddress.hostAddress})"
                startForegroundNotification(pairCode, verifiedHostAddress.hostAddress ?: "")
                broadcastState(connected = true, connecting = false, statusMsg = currentStatusMessage)

                val pfd = vpnInterface!!
                val inputStream = FileInputStream(pfd.fileDescriptor)
                val outputStream = FileOutputStream(pfd.fileDescriptor)

                val outBuffer = ByteArray(32768)
                var seq = 1L
                val activeSocket = clientSocket
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
                Log.e(TAG, "VPN Tunnel Error", e)
                lastErrorMessage = e.message ?: "Failed to connect to Host #$pairCode"
                broadcastState(connected = false, connecting = false, error = lastErrorMessage)
                stopVpn()
            } finally {
                clientSocket?.close()
            }
        }
    }

    private fun startForegroundNotification(pairCode: String, hostAddress: String) {
        val channelId = "aura_mesh_vpn_channel"
        val notificationManager = getSystemService(NotificationManager::class.java)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Aura 5G Mesh Tunnel",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows live telemetry of active 5G Mesh Tunnel"
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
            .setContentText("Routed via Host $hostAddress. 100% OS traffic protected.")
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

    private fun broadcastState(connected: Boolean, connecting: Boolean, statusMsg: String? = null, error: String? = null) {
        val intent = Intent(ACTION_STATE_CHANGED).apply {
            putExtra(EXTRA_IS_CONNECTED, connected)
            putExtra(EXTRA_IS_CONNECTING, connecting)
            if (statusMsg != null) putExtra(EXTRA_STATUS_MSG, statusMsg)
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
