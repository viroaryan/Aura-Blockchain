package org.auranetwork.mesh

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.PowerSettingsNew
import androidx.compose.material.icons.filled.Radio
import androidx.compose.material.icons.filled.SwapHoriz
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay

class MainActivity : ComponentActivity() {

    private var targetPairCode by mutableStateOf("849201")
    private var isConnected by mutableStateOf(false)
    private var dataServedMb by mutableStateOf(0.0)

    private val vpnPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            startAuraVpn()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AuraMeshAppTheme {
                MainScreen()
            }
        }
    }

    @Composable
    fun MainScreen() {
        LaunchedEffect(isConnected) {
            while (isConnected) {
                delay(1000)
                val bytes = AuraVpnService.bytesTransferred.get()
                dataServedMb = (bytes / (1024.0 * 1024.0))
            }
        }

        Surface(
            modifier = Modifier.fillMaxSize(),
            color = Color(0xFFF8FAFC)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.SpaceBetween
            ) {
                // Header
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Spacer(modifier = Modifier.height(16.dp))
                    Box(
                        modifier = Modifier
                            .size(64.dp)
                            .background(
                                brush = Brush.linearGradient(
                                    listOf(Color(0xFF059669), Color(0xFF10B981))
                                ),
                                shape = RoundedCornerShape(20.dp)
                            ),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Radio,
                            contentDescription = "Aura Logo",
                            tint = Color.White,
                            modifier = Modifier.size(36.dp)
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = "Aura Mesh 5G Tunnel",
                        fontSize = 24.sp,
                        fontWeight = FontWeight.ExtraBold,
                        color = Color(0xFF0F172A)
                    )
                    Text(
                        text = "Zero-Native Data Remote Routing Engine",
                        fontSize = 12.sp,
                        color = Color(0xFF64748B)
                    )
                }

                // Middle Card: Connection Controls
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(24.dp),
                    colors = CardDefaults.cardColors(containerColor = Color.White),
                    elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(20.dp),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Text(
                            text = if (isConnected) "TUNNEL ACTIVE (100% ROUTED)" else "DISCONNECTED",
                            fontWeight = FontWeight.Bold,
                            fontSize = 13.sp,
                            color = if (isConnected) Color(0xFF059669) else Color(0xFF94A3B8),
                            fontFamily = FontFamily.Monospace
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        if (!isConnected) {
                            OutlinedTextField(
                                value = targetPairCode,
                                onValueChange = { if (it.length <= 6) targetPairCode = it },
                                label = { Text("Enter 6-Digit Pair Code") },
                                singleLine = true,
                                shape = RoundedCornerShape(16.dp),
                                modifier = Modifier.fillMaxWidth()
                            )
                        } else {
                            Text(
                                text = "%.2f MB".format(dataServedMb),
                                fontSize = 36.sp,
                                fontWeight = FontWeight.ExtraBold,
                                color = Color(0xFF059669),
                                fontFamily = FontFamily.Monospace
                            )
                            Text(
                                text = "Routed via 5G Node #$targetPairCode",
                                fontSize = 12.sp,
                                color = Color(0xFF64748B)
                            )
                        }

                        Spacer(modifier = Modifier.height(20.dp))

                        Button(
                            onClick = {
                                if (isConnected) {
                                    stopAuraVpn()
                                } else {
                                    prepareAndStartVpn()
                                }
                            },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(56.dp),
                            shape = RoundedCornerShape(16.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (isConnected) Color(0xFFEF4444) else Color(0xFF059669)
                            )
                        ) {
                            Icon(
                                imageVector = Icons.Default.PowerSettingsNew,
                                contentDescription = "Toggle VPN",
                                modifier = Modifier.size(20.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = if (isConnected) "Disconnect Tunnel" else "Connect to 5G Hotspot",
                                fontWeight = FontWeight.Bold,
                                fontSize = 15.sp
                            )
                        }
                    }
                }

                // Bottom Status
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(Color(0xFFF1F5F9), RoundedCornerShape(16.dp))
                        .padding(14.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Lock,
                            contentDescription = "Lock",
                            tint = Color(0xFF059669),
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(
                            text = "ChaCha20-Poly1305",
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF334155),
                            fontFamily = FontFamily.Monospace
                        )
                    }

                    Text(
                        text = "VpnService Active",
                        fontSize = 11.sp,
                        color = Color(0xFF64748B),
                        fontFamily = FontFamily.Monospace
                    )
                }
            }
        }
    }

    private fun prepareAndStartVpn() {
        val intent = VpnService.prepare(this)
        if (intent != null) {
            vpnPermissionLauncher.launch(intent)
        } else {
            startAuraVpn()
        }
    }

    private fun startAuraVpn() {
        val intent = Intent(this, AuraVpnService::class.java).apply {
            action = AuraVpnService.ACTION_CONNECT
            putExtra(AuraVpnService.EXTRA_PAIR_CODE, targetPairCode)
            putExtra(AuraVpnService.EXTRA_HOST_IP, "103.21.244.18")
            putExtra(AuraVpnService.EXTRA_HOST_PORT, 9000)
        }
        startService(intent)
        isConnected = true
    }

    private fun stopAuraVpn() {
        val intent = Intent(this, AuraVpnService::class.java).apply {
            action = AuraVpnService.ACTION_DISCONNECT
        }
        startService(intent)
        isConnected = false
    }
}

@Composable
fun AuraMeshAppTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = lightColorScheme(
            primary = Color(0xFF059669),
            secondary = Color(0xFF10B981),
            background = Color(0xFFF8FAFC)
        ),
        content = content
    )
}
