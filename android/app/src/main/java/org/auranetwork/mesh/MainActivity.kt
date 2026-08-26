package org.auranetwork.mesh

import android.Manifest
import android.app.Activity
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.net.VpnService
import android.os.Build
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import kotlinx.coroutines.delay

class MainActivity : ComponentActivity() {

    private var targetPairCode by mutableStateOf("849201")
    private var myHostCode by mutableStateOf("")
    private var isConnected by mutableStateOf(false)
    private var isHostMode by mutableStateOf(false)
    private var dataServedMb by mutableStateOf(0.0)
    private var errorMessage by mutableStateOf<String?>(null)

    // VPN Permission Launcher
    private val vpnPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            startAuraVpn()
        } else {
            Toast.makeText(this, "VPN permission is required to route traffic", Toast.LENGTH_LONG).show()
        }
    }

    // Android 13+ Notification Permission Launcher
    private val notificationPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        if (!isGranted) {
            Toast.makeText(this, "Notification permission recommended for background status", Toast.LENGTH_SHORT).show()
        }
    }

    private val vpnStateReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val connected = intent?.getBooleanExtra(AuraVpnService.EXTRA_IS_CONNECTED, false) ?: false
            val error = intent?.getStringExtra(AuraVpnService.EXTRA_ERROR)
            isConnected = connected
            if (error != null) {
                errorMessage = error
                Toast.makeText(this@MainActivity, error, Toast.LENGTH_LONG).show()
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Generate persistent local host code
        myHostCode = (100000 + (Math.random() * 900000).toInt()).toString()
        isConnected = AuraVpnService.isConnectedState.get()

        // Request notification permission on Android 13+
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }

        // Register State Broadcast Receiver
        val filter = IntentFilter(AuraVpnService.ACTION_STATE_CHANGED)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(vpnStateReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(vpnStateReceiver, filter)
        }

        setContent {
            AuraMeshAppTheme {
                MainScreen()
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        try {
            unregisterReceiver(vpnStateReceiver)
        } catch (e: Exception) {
            // ignore unregister error
        }
    }

    @OptIn(ExperimentalMaterial3Api::class)
    @Composable
    fun MainScreen() {
        val clipboardManager = LocalClipboardManager.current

        LaunchedEffect(isConnected) {
            while (isConnected) {
                delay(1000)
                val bytes = AuraVpnService.bytesTransferred.get()
                dataServedMb = (bytes / (1024.0 * 1024.0))
            }
        }

        Scaffold(
            topBar = {
                TopAppBar(
                    title = {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Box(
                                modifier = Modifier
                                    .size(36.dp)
                                    .background(
                                        brush = Brush.linearGradient(
                                            listOf(Color(0xFF059669), Color(0xFF10B981))
                                        ),
                                        shape = RoundedCornerShape(10.dp)
                                    ),
                                contentAlignment = Alignment.Center
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Radio,
                                    contentDescription = "Logo",
                                    tint = Color.White,
                                    modifier = Modifier.size(20.dp)
                                )
                            }
                            Spacer(modifier = Modifier.width(10.dp))
                            Column {
                                Text(
                                    text = "Aura Mesh 5G",
                                    fontSize = 18.sp,
                                    fontWeight = FontWeight.ExtraBold,
                                    color = Color(0xFF0F172A)
                                )
                                Text(
                                    text = "Encrypted dVPN Hotspot",
                                    fontSize = 10.sp,
                                    color = Color(0xFF64748B)
                                )
                            }
                        }
                    },
                    actions = {
                        Surface(
                            shape = CircleShape,
                            color = if (isConnected) Color(0xFFECFDF5) else Color(0xFFF1F5F9),
                            modifier = Modifier.padding(end = 12.dp)
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(8.dp)
                                        .background(
                                            if (isConnected) Color(0xFF10B981) else Color(0xFF94A3B8),
                                            CircleShape
                                        )
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = if (isConnected) "ONLINE" else "IDLE",
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.Bold,
                                    fontFamily = FontFamily.Monospace,
                                    color = if (isConnected) Color(0xFF047857) else Color(0xFF64748B)
                                )
                            }
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = Color(0xFFF8FAFC))
                )
            },
            containerColor = Color(0xFFF8FAFC)
        ) { paddingValues ->
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
                    .padding(horizontal = 20.dp, vertical = 8.dp),
                verticalArrangement = Arrangement.SpaceBetween
            ) {
                // Mode Toggle Bar (Consumer vs Host)
                Card(
                    shape = RoundedCornerShape(16.dp),
                    colors = CardDefaults.cardColors(containerColor = Color(0xFFE2E8F0)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier
                            .padding(4.dp)
                            .fillMaxWidth()
                    ) {
                        Surface(
                            modifier = Modifier
                                .weight(1f)
                                .clip(RoundedCornerShape(12.dp))
                                .clickable { isHostMode = false },
                            color = if (!isHostMode) Color.White else Color.Transparent,
                            shadowElevation = if (!isHostMode) 2.dp else 0.dp
                        ) {
                            Row(
                                modifier = Modifier.padding(vertical = 10.dp),
                                horizontalArrangement = Arrangement.Center,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Wifi,
                                    contentDescription = "Use",
                                    modifier = Modifier.size(16.dp),
                                    tint = if (!isHostMode) Color(0xFF059669) else Color(0xFF64748B)
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = "Use 5G Hotspot",
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = if (!isHostMode) Color(0xFF0F172A) else Color(0xFF64748B)
                                )
                            }
                        }

                        Surface(
                            modifier = Modifier
                                .weight(1f)
                                .clip(RoundedCornerShape(12.dp))
                                .clickable { isHostMode = true },
                            color = if (isHostMode) Color.White else Color.Transparent,
                            shadowElevation = if (isHostMode) 2.dp else 0.dp
                        ) {
                            Row(
                                modifier = Modifier.padding(vertical = 10.dp),
                                horizontalArrangement = Arrangement.Center,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Share,
                                    contentDescription = "Share",
                                    modifier = Modifier.size(16.dp),
                                    tint = if (isHostMode) Color(0xFF4F46E5) else Color(0xFF64748B)
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = "Share My 5G",
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = if (isHostMode) Color(0xFF0F172A) else Color(0xFF64748B)
                                )
                            }
                        }
                    }
                }

                // Error Banner if any
                if (errorMessage != null) {
                    Card(
                        shape = RoundedCornerShape(14.dp),
                        colors = CardDefaults.cardColors(containerColor = Color(0xFFFEF2F2)),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier.padding(12.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = Icons.Default.Warning,
                                contentDescription = "Error",
                                tint = Color(0xFFDC2626),
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = errorMessage ?: "",
                                fontSize = 12.sp,
                                color = Color(0xFF991B1B)
                            )
                        }
                    }
                }

                // Main Action Card
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(24.dp),
                    colors = CardDefaults.cardColors(containerColor = Color.White),
                    elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(24.dp),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        if (!isHostMode) {
                            // Consumer Mode (Priti)
                            Text(
                                text = if (isConnected) "● 100% OS TRAFFIC ROUTED" else "○ TUNNEL DISCONNECTED",
                                fontWeight = FontWeight.ExtraBold,
                                fontSize = 12.sp,
                                color = if (isConnected) Color(0xFF059669) else Color(0xFF94A3B8),
                                fontFamily = FontFamily.Monospace
                            )

                            Spacer(modifier = Modifier.height(18.dp))

                            if (isConnected) {
                                Text(
                                    text = "%.2f MB".format(dataServedMb),
                                    fontSize = 42.sp,
                                    fontWeight = FontWeight.ExtraBold,
                                    color = Color(0xFF059669),
                                    fontFamily = FontFamily.Monospace
                                )
                                Text(
                                    text = "Live Data via 5G Peer #$targetPairCode",
                                    fontSize = 12.sp,
                                    color = Color(0xFF64748B)
                                )
                            } else {
                                OutlinedTextField(
                                    value = targetPairCode,
                                    onValueChange = { if (it.length <= 6) targetPairCode = it },
                                    label = { Text("Enter Host's 6-Digit Code") },
                                    placeholder = { Text("e.g. 849201") },
                                    singleLine = true,
                                    textStyle = LocalTextStyle.current.copy(
                                        textAlign = TextAlign.Center,
                                        fontSize = 20.sp,
                                        fontFamily = FontFamily.Monospace,
                                        fontWeight = FontWeight.Bold
                                    ),
                                    shape = RoundedCornerShape(16.dp),
                                    modifier = Modifier.fillMaxWidth()
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
                                    .height(54.dp),
                                shape = RoundedCornerShape(16.dp),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = if (isConnected) Color(0xFFEF4444) else Color(0xFF059669)
                                )
                            ) {
                                Icon(
                                    imageVector = if (isConnected) Icons.Default.Close else Icons.Default.PowerSettingsNew,
                                    contentDescription = "Toggle",
                                    modifier = Modifier.size(20.dp)
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(
                                    text = if (isConnected) "Disconnect 5G Tunnel" else "Connect to 5G Hotspot",
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 15.sp
                                )
                            }
                        } else {
                            // Host Mode (Aryan)
                            Text(
                                text = "YOUR 5G HOTSPOT CODE",
                                fontWeight = FontWeight.Bold,
                                fontSize = 12.sp,
                                color = Color(0xFF4F46E5),
                                fontFamily = FontFamily.Monospace
                            )

                            Spacer(modifier = Modifier.height(14.dp))

                            Surface(
                                shape = RoundedCornerShape(18.dp),
                                color = Color(0xFFEEF2FF),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
                                        clipboardManager.setText(AnnotatedString(myHostCode))
                                        Toast.makeText(this@MainActivity, "Host code copied to clipboard!", Toast.LENGTH_SHORT).show()
                                    }
                            ) {
                                Column(
                                    modifier = Modifier.padding(18.dp),
                                    horizontalAlignment = Alignment.CenterHorizontally
                                ) {
                                    Text(
                                        text = myHostCode,
                                        fontSize = 36.sp,
                                        fontWeight = FontWeight.ExtraBold,
                                        color = Color(0xFF4338CA),
                                        fontFamily = FontFamily.Monospace,
                                        letterSpacing = 6.sp
                                    )
                                    Spacer(modifier = Modifier.height(4.dp))
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Icon(
                                            imageVector = Icons.Default.ContentCopy,
                                            contentDescription = "Copy",
                                            tint = Color(0xFF6366F1),
                                            modifier = Modifier.size(14.dp)
                                        )
                                        Spacer(modifier = Modifier.width(4.dp))
                                        Text(
                                            text = "Tap to Copy Code",
                                            fontSize = 11.sp,
                                            color = Color(0xFF6366F1),
                                            fontWeight = FontWeight.SemiBold
                                        )
                                    }
                                }
                            }

                            Spacer(modifier = Modifier.height(14.dp))
                            Text(
                                text = "Tell your friend to enter this 6-digit code in their app. Their phone's internet will be seamlessly routed through your 5G network!",
                                fontSize = 11.sp,
                                color = Color(0xFF64748B),
                                textAlign = TextAlign.Center,
                                lineHeight = 16.sp
                            )
                        }
                    }
                }

                // Security & Privacy Guarantee Footer
                Card(
                    shape = RoundedCornerShape(16.dp),
                    colors = CardDefaults.cardColors(containerColor = Color(0xFFF1F5F9)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier.padding(14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                imageVector = Icons.Default.Shield,
                                contentDescription = "Shield",
                                tint = Color(0xFF059669),
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = "AES-256-GCM / Noise IK",
                                fontSize = 11.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color(0xFF334155),
                                fontFamily = FontFamily.Monospace
                            )
                        }

                        Text(
                            text = "Zero Logs",
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF059669),
                            fontFamily = FontFamily.Monospace
                        )
                    }
                }
            }
        }
    }

    private fun prepareAndStartVpn() {
        val target = targetPairCode.trim()
        if (target.length < 5) {
            Toast.makeText(this, "Please enter a valid 6-digit pair code", Toast.LENGTH_SHORT).show()
            return
        }

        errorMessage = null
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
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
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
            background = Color(0xFFF8FAFC),
            surface = Color.White
        ),
        content = content
    )
}
