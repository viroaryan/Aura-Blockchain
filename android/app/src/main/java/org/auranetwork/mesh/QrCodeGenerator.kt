package org.auranetwork.mesh

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.abs

/**
 * Pure Kotlin QR Code Matrix Generator & Compose Canvas Renderer
 * Generates a deterministic 21x21 QR-like matrix for 6-digit Aura pairing codes.
 */
object QrCodeGenerator {

    fun generateMatrix(data: String): Array<BooleanArray> {
        val size = 21
        val matrix = Array(size) { BooleanArray(size) { false } }

        // Draw Finder Patterns (Top-Left, Top-Right, Bottom-Left)
        drawFinderPattern(matrix, 0, 0)
        drawFinderPattern(matrix, size - 7, 0)
        drawFinderPattern(matrix, 0, size - 7)

        // Draw Timing Patterns
        for (i in 8 until size - 8) {
            matrix[6][i] = (i % 2 == 0)
            matrix[i][6] = (i % 2 == 0)
        }

        // Draw Data Bits from Hash of data
        val hash = abs(data.hashCode())
        var bitIndex = 0
        for (r in 0 until size) {
            for (c in 0 until size) {
                // Skip finder patterns
                if ((r < 8 && c < 8) || (r < 8 && c >= size - 8) || (r >= size - 8 && c < 8)) continue
                if (r == 6 || c == 6) continue

                val isBitSet = ((hash shr (bitIndex % 31)) and 1) == 1
                matrix[r][c] = isBitSet xor ((r + c) % 2 == 0)
                bitIndex++
            }
        }

        return matrix
    }

    private fun drawFinderPattern(matrix: Array<BooleanArray>, startR: Int, startC: Int) {
        for (r in 0 until 7) {
            for (c in 0 until 7) {
                if (r == 0 || r == 6 || c == 0 || c == 6) {
                    matrix[startR + r][startC + c] = true
                } else if (r in 2..4 && c in 2..4) {
                    matrix[startR + r][startC + c] = true
                }
            }
        }
    }
}

@Composable
fun QrCodeView(
    data: String,
    modifier: Modifier = Modifier,
    size: Dp = 160.dp,
    dotColor: Color = Color(0xFF0F172A),
    backgroundColor: Color = Color.White
) {
    val matrix = QrCodeGenerator.generateMatrix(data)
    val matrixSize = matrix.size

    Box(
        modifier = modifier
            .size(size)
            .background(backgroundColor, RoundedCornerShape(12.dp)),
        contentAlignment = Alignment.Center
    ) {
        Canvas(modifier = Modifier.size(size - 16.dp)) {
            val moduleWidth = this.size.width / matrixSize
            val moduleHeight = this.size.height / matrixSize

            for (r in 0 until matrixSize) {
                for (c in 0 until matrixSize) {
                    if (matrix[r][c]) {
                        drawRect(
                            color = dotColor,
                            topLeft = Offset(c * moduleWidth, r * moduleHeight),
                            size = Size(moduleWidth, moduleHeight)
                        )
                    }
                }
            }
        }
    }
}
