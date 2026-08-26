package org.auranetwork.mesh

import java.security.MessageDigest
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

object CryptoEngine {
    private const val DOMAIN_TAG = "AURA_AEAD_TAG_v1"

    fun deriveSessionKey(clientEphemeral: ByteArray, hostEphemeral: ByteArray): ByteArray {
        val digest = MessageDigest.getInstance("SHA-256")
        digest.update(clientEphemeral)
        digest.update(hostEphemeral)
        return digest.digest()
    }

    fun encryptPacket(plaintext: ByteArray, sessionKey: ByteArray, seq: Long): ByteArray {
        val keySpec = SecretKeySpec(sessionKey, 0, 32, "AES")
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        
        // 12-byte IV derived from sequence ID
        val iv = ByteArray(12)
        for (i in 0..7) {
            iv[i] = ((seq ushr (56 - i * 8)) and 0xFF).toByte()
        }
        
        val gcmSpec = GCMParameterSpec(128, iv)
        cipher.init(Cipher.ENCRYPT_MODE, keySpec, gcmSpec)
        
        val ciphertext = cipher.doFinal(plaintext)
        
        // Output frame: seq (8 bytes) + ciphertext (includes 16-byte auth tag)
        val frame = ByteArray(8 + ciphertext.size)
        for (i in 0..7) {
            frame[i] = ((seq ushr (56 - i * 8)) and 0xFF).toByte()
        }
        System.arraycopy(ciphertext, 0, frame, 8, ciphertext.size)
        return frame
    }

    fun decryptPacket(frame: ByteArray, sessionKey: ByteArray): ByteArray {
        if (frame.size < 24) throw IllegalArgumentException("Frame too short")
        
        var seq: Long = 0
        for (i in 0..7) {
            seq = (seq shl 8) or (frame[i].toLong() and 0xFF)
        }
        
        val iv = ByteArray(12)
        for (i in 0..7) {
            iv[i] = ((seq ushr (56 - i * 8)) and 0xFF).toByte()
        }
        
        val keySpec = SecretKeySpec(sessionKey, 0, 32, "AES")
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        val gcmSpec = GCMParameterSpec(128, iv)
        cipher.init(Cipher.DECRYPT_MODE, keySpec, gcmSpec)
        
        return cipher.doFinal(frame, 8, frame.size - 8)
    }
}
