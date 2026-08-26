package org.auranetwork.mesh

import java.net.InetAddress
import java.nio.ByteBuffer

/**
 * Fast binary IPv4/TCP/UDP packet parser and builder for Android TUN interface.
 */
object IpPacketParser {

    const val PROTOCOL_ICMP = 1
    const val PROTOCOL_TCP = 6
    const val PROTOCOL_UDP = 17

    data class ParsedPacket(
        val version: Int,
        val protocol: Int,
        val sourceIp: InetAddress,
        val destIp: InetAddress,
        val sourcePort: Int,
        val destPort: Int,
        val payload: ByteArray,
        val rawPacket: ByteArray
    )

    fun parse(packet: ByteArray): ParsedPacket? {
        if (packet.size < 20) return null

        val versionAndIhl = packet[0].toInt() and 0xFF
        val version = versionAndIhl shr 4
        if (version != 4) return null // Only IPv4 supported in this pass

        val ihl = (versionAndIhl and 0x0F) * 4
        if (packet.size < ihl) return null

        val protocol = packet[9].toInt() and 0xFF

        val srcIpBytes = ByteArray(4) { packet[12 + it] }
        val dstIpBytes = ByteArray(4) { packet[16 + it] }
        val srcIp = InetAddress.getByAddress(srcIpBytes)
        val dstIp = InetAddress.getByAddress(dstIpBytes)

        var srcPort = 0
        var dstPort = 0
        var payloadOffset = ihl

        if (protocol == PROTOCOL_TCP && packet.size >= ihl + 20) {
            srcPort = ((packet[ihl].toInt() and 0xFF) shl 8) or (packet[ihl + 1].toInt() and 0xFF)
            dstPort = ((packet[ihl + 2].toInt() and 0xFF) shl 8) or (packet[ihl + 3].toInt() and 0xFF)
            val dataOffset = ((packet[ihl + 12].toInt() and 0xF0) shr 4) * 4
            payloadOffset = ihl + dataOffset
        } else if (protocol == PROTOCOL_UDP && packet.size >= ihl + 8) {
            srcPort = ((packet[ihl].toInt() and 0xFF) shl 8) or (packet[ihl + 1].toInt() and 0xFF)
            dstPort = ((packet[ihl + 2].toInt() and 0xFF) shl 8) or (packet[ihl + 3].toInt() and 0xFF)
            payloadOffset = ihl + 8
        }

        val payloadSize = (packet.size - payloadOffset).coerceAtLeast(0)
        val payload = ByteArray(payloadSize)
        if (payloadSize > 0 && payloadOffset + payloadSize <= packet.size) {
            System.arraycopy(packet, payloadOffset, payload, 0, payloadSize)
        }

        return ParsedPacket(
            version = version,
            protocol = protocol,
            sourceIp = srcIp,
            destIp = dstIp,
            sourcePort = srcPort,
            destPort = dstPort,
            payload = payload,
            rawPacket = packet
        )
    }

    /**
     * Constructs a valid IPv4 UDP packet
     */
    fun buildUdpPacket(
        srcIp: InetAddress,
        dstIp: InetAddress,
        srcPort: Int,
        dstPort: Int,
        payload: ByteArray
    ): ByteArray {
        val totalLength = 20 + 8 + payload.size
        val packet = ByteBuffer.allocate(totalLength)

        // IP Header (20 bytes)
        packet.put(0x45.toByte()) // Version 4, IHL 5
        packet.put(0x00.toByte()) // DSCP / ECN
        packet.putShort(totalLength.toShort()) // Total Length
        packet.putShort(0x0000.toShort()) // Identification
        packet.putShort(0x4000.toShort()) // Flags (Don't Fragment) + Fragment Offset
        packet.put(64.toByte()) // TTL
        packet.put(PROTOCOL_UDP.toByte()) // Protocol
        packet.putShort(0.toShort()) // Checksum placeholder
        packet.put(srcIp.address)
        packet.put(dstIp.address)

        // Calculate and insert IP checksum
        val ipChecksum = calculateChecksum(packet.array(), 0, 20)
        packet.putShort(10, ipChecksum.toShort())

        // UDP Header (8 bytes)
        packet.position(20)
        packet.putShort(srcPort.toShort())
        packet.putShort(dstPort.toShort())
        packet.putShort((8 + payload.size).toShort())
        packet.putShort(0.toShort()) // UDP Checksum optional in IPv4

        // Payload
        packet.put(payload)

        return packet.array()
    }

    private fun calculateChecksum(data: ByteArray, offset: Int, length: Int): Int {
        var sum = 0
        var i = offset
        while (i < offset + length - 1) {
            val word = ((data[i].toInt() and 0xFF) shl 8) or (data[i + 1].toInt() and 0xFF)
            sum += word
            i += 2
        }
        if (i < offset + length) {
            sum += (data[i].toInt() and 0xFF) shl 8
        }
        while ((sum shr 16) > 0) {
            sum = (sum and 0xFFFF) + (sum shr 16)
        }
        return (sum.inv()) and 0xFFFF
    }
}
