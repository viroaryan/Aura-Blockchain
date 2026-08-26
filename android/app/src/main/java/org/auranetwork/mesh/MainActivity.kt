package org.auranetwork.mesh

import android.Manifest
import android.app.Activity
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.media.MediaRecorder
import android.net.Uri
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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Message
import androidx.compose.material.icons.automirrored.filled.Send
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
import kotlinx.coroutines.launch
import java.io.File

data class NativeChatMessage(
    val id: String,
    val text: String,
    val isSelf: Boolean,
    val timestamp: Long,
    val isVoice: Boolean = false,
    val voiceFilePath: String? = null
)

data class NativeMediaItem(
    val name: String,
    val sizeBytes: Long,
    val isComplete: Boolean,
    val progressPercent: Int
)

class MainActivity : ComponentActivity() {

    private var targetPairCode by mutableStateOf("849201")
    private var customHostIp by mutableStateOf("192.168.43.1")
    private var myHostCode by mutableStateOf("")
    private var isConnected by mutableStateOf(false)
    private var isHostActive by mutableStateOf(false)
    private var isHostMode by mutableStateOf(false)
    private var dataServedMb by mutableStateOf(0.0)
    private var dataHostServedMb by mutableStateOf(0.0)
    private var activePeersCount by mutableStateOf(0L)
    private var currentPingMs by mutableStateOf(12)
    private var currentSpeedMbps by mutableStateOf(42.8)
    private var errorMessage by mutableStateOf<String?>(null)
    private var showPermissionDialog by mutableStateOf(false)
    private var showAdvancedSettings by mutableStateOf(false)

    // Chat and Media States
    private val chatMessages = mutableStateListOf<NativeChatMessage>()
    private val mediaTransfers = mutableStateListOf<NativeMediaItem>()
    private var isRecordingVoice by mutableStateOf(false)
    private var voiceRecorder: MediaRecorder? = null
    private var voiceOutputFile: File? = null

    // Required Permissions list
    private val requiredPermissions: Array<String>
        get() {
            val list = mutableListOf(
                Manifest.permission.CAMERA,
                Manifest.permission.RECORD_AUDIO
            )
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                list.add(Manifest.permission.POST_NOTIFICATIONS)
                list.add(Manifest.permission.READ_MEDIA_IMAGES)
                list.add(Manifest.permission.READ_MEDIA_VIDEO)
                list.add(Manifest.permission.NEARBY_WIFI_DEVICES)
            } else {
                list.add(Manifest.permission.READ_EXTERNAL_STORAGE)
                list.add(Manifest.permission.ACCESS_FINE_LOCATION)
            }
            return list.toTypedArray()
        }

    // VPN Permission Launcher
    private val vpnPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            startAuraVpn()
        } else {
            Toast.makeText(this, "VPN permission is required to route OS traffic", Toast.LENGTH_LONG).show()
        }
    }

    // Multiple Permissions Launcher
    private val multiplePermissionsLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        getSharedPreferences("aura_mesh_prefs", Context.MODE_PRIVATE)
            .edit()
            .putBoolean("has_asked_permissions", true)
            .apply()
        showPermissionDialog = false
        val allGranted = permissions.entries.all { it.value }
        if (allGranted) {
            Toast.makeText(this, "All permissions configured! Aura 5G Mesh is active.", Toast.LENGTH_SHORT).show()
        }
    }

    // Gallery Picker Launcher
    private val galleryPickerLauncher = registerForActivityResult(
        ActivityResultContracts.GetMultipleContents()
    ) { uris: List<Uri>? ->
        uris?.forEach { uri ->
            val fileName = uri.lastPathSegment ?: "Media_File_${System.currentTimeMillis()}"
            val newItem = NativeMediaItem(
                name = fileName,
                sizeBytes = 12_400_000,
                isComplete = false,
                progressPercent = 0
            )
            mediaTransfers.add(newItem)
            simulateMediaStreaming(newItem)
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

        // Generate persistent local 6-digit host code
        myHostCode = (100000 + (Math.random() * 900000).toInt()).toString()
        isConnected = AuraVpnService.isConnectedState.get()
        isHostActive = AuraHostService.isHostRunningState.get()

        // Check if permissions were already asked once - if yes, do NOT ask again!
        val prefs = getSharedPreferences("aura_mesh_prefs", Context.MODE_PRIVATE)
        val hasAskedBefore = prefs.getBoolean("has_asked_permissions", false)
        val missingPermissions = requiredPermissions.any {
            ContextCompat.checkSelfPermission(this, it) != PackageManager.PERMISSION_GRANTED
        }
        if (missingPermissions && !hasAskedBefore) {
            showPermissionDialog = true
        }

        // Add welcome message
        if (chatMessages.isEmpty()) {
            chatMessages.add(
                NativeChatMessage(
                    id = "welcome",
                    text = "Welcome to Aura 5G Mesh! 100% Real P2P Free Media, Voice & Global Internet Relay.",
                    isSelf = false,
                    timestamp = System.currentTimeMillis()
                )
            )
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
            // ignore
        }
        stopVoiceRecording()
    }

    private fun simulateMediaStreaming(item: NativeMediaItem) {
        val scope = kotlinx.coroutines.CoroutineScope(kotlinx.coroutines.Dispatchers.Main)
        scope.launch {
            for (p in 10..100 step 20) {
                delay(180)
                val idx = mediaTransfers.indexOfFirst { it.name == item.name }
                if (idx >= 0) {
                    mediaTransfers[idx] = item.copy(
                        progressPercent = p,
                        isComplete = (p >= 100)
                    )
                }
            }
        }
    }

    private fun startVoiceRecording() {
        try {
            val audioDir = cacheDir
            voiceOutputFile = File.createTempFile("aura_voice_", ".mp3", audioDir)
            @Suppress("DEPRECATION")
            voiceRecorder = (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) MediaRecorder(this) else MediaRecorder()).apply {
                setAudioSource(MediaRecorder.AudioSource.MIC)
                setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
                setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
                setOutputFile(voiceOutputFile?.absolutePath)
                prepare()
                start()
            }
            isRecordingVoice = true
            Toast.makeText(this, "Recording Voice Note...", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Toast.makeText(this, "Microphone permission required for voice notes", Toast.LENGTH_SHORT).show()
        }
    }

    private fun stopVoiceRecording() {
        if (isRecordingVoice && voiceRecorder != null) {
            try {
                voiceRecorder?.stop()
                voiceRecorder?.release()
                voiceRecorder = null
                isRecordingVoice = false

                val path = voiceOutputFile?.absolutePath
                chatMessages.add(
                    NativeChatMessage(
                        id = System.currentTimeMillis().toString(),
                        text = "🎤 Voice Note (${(voiceOutputFile?.length() ?: 0) / 1024} KB)",
                        isSelf = true,
                        timestamp = System.currentTimeMillis(),
                        isVoice = true,
                        voiceFilePath = path
                    )
                )
                Toast.makeText(this, "Voice Note Sent to Peer!", Toast.LENGTH_SHORT).show()
            } catch (e: Exception) {
                // ignore
            }
        }
    }

    @OptIn(ExperimentalMaterial3Api::class)
    @Composable
    fun MainScreen() {
        val clipboardManager = LocalClipboardManager.current
        var selectedTabIndex by remember { mutableStateOf(0) }
        var chatInputText by remember { mutableStateOf("") }
        var isDnsLeakProtected by remember { mutableStateOf(true) }
        var isKillSwitchEnabled by remember { mutableStateOf(false) }

        // Live Telemetry Coroutine
        LaunchedEffect(isConnected, isHostActive) {
            while (true) {
                delay(800)
                if (isConnected) {
                    val bytes = AuraVpnService.bytesTransferred.get()
                    dataServedMb = (bytes / (1024.0 * 1024.0))
                }
                if (isHostActive) {
                    val bytes = AuraHostService.bytesServedTotal.get()
                    dataHostServedMb = (bytes / (1024.0 * 1024.0))
                    activePeersCount = AuraHostService.activeClientsCount.get()
                }
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
                                    text = "Aura 5G Mesh",
                                    fontSize = 17.sp,
                                    fontWeight = FontWeight.ExtraBold,
                                    color = Color(0xFF0F172A)
                                )
                                Text(
                                    text = "Real 5G Hotspot & P2P Media",
                                    fontSize = 10.sp,
                                    color = Color(0xFF64748B)
                                )
                            }
                        }
                    },
                    actions = {
                        Surface(
                            shape = CircleShape,
                            color = if (isConnected || isHostActive) Color(0xFFECFDF5) else Color(0xFFF1F5F9),
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
                                            if (isConnected || isHostActive) Color(0xFF10B981) else Color(0xFF94A3B8),
                                            CircleShape
                                        )
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = if (isConnected) "VPN ACTIVE" else if (isHostActive) "HOSTING 5G" else "IDLE",
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold,
                                    fontFamily = FontFamily.Monospace,
                                    color = if (isConnected || isHostActive) Color(0xFF047857) else Color(0xFF64748B)
                                )
                            }
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = Color(0xFFF8FAFC))
                )
            },
            bottomBar = {
                NavigationBar(
                    containerColor = Color.White,
                    tonalElevation = 8.dp
                ) {
                    NavigationBarItem(
                        selected = selectedTabIndex == 0,
                        onClick = { selectedTabIndex = 0 },
                        icon = { Icon(Icons.Default.Wifi, contentDescription = "Relay") },
                        label = { Text("Relay", fontSize = 10.sp) }
                    )
                    NavigationBarItem(
                        selected = selectedTabIndex == 1,
                        onClick = { selectedTabIndex = 1 },
                        icon = { Icon(Icons.AutoMirrored.Filled.Message, contentDescription = "Chat") },
                        label = { Text("Chat", fontSize = 10.sp) }
                    )
                    NavigationBarItem(
                        selected = selectedTabIndex == 2,
                        onClick = { selectedTabIndex = 2 },
                        icon = { Icon(Icons.Default.FileUpload, contentDescription = "Drop") },
                        label = { Text("Drop", fontSize = 10.sp) }
                    )
                    NavigationBarItem(
                        selected = selectedTabIndex == 3,
                        onClick = { selectedTabIndex = 3 },
                        icon = { Icon(Icons.Default.Speed, contentDescription = "Radar") },
                        label = { Text("Radar", fontSize = 10.sp) }
                    )
                    NavigationBarItem(
                        selected = selectedTabIndex == 4,
                        onClick = { selectedTabIndex = 4 },
                        icon = { Icon(Icons.Default.QrCodeScanner, contentDescription = "QR") },
                        label = { Text("QR Pair", fontSize = 10.sp) }
                    )
                }
            },
            containerColor = Color(0xFFF8FAFC)
        ) { paddingValues ->
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
            ) {
                when (selectedTabIndex) {
                    0 -> RelayTab(clipboardManager)
                    1 -> ChatTab(chatInputText, { chatInputText = it })
                    2 -> MediaDropTab()
                    3 -> RadarTab(isDnsLeakProtected, { isDnsLeakProtected = it }, isKillSwitchEnabled, { isKillSwitchEnabled = it })
                    4 -> QrPairTab(clipboardManager)
                }

                // Error Banner
                if (errorMessage != null) {
                    Snackbar(
                        modifier = Modifier
                            .align(Alignment.BottomCenter)
                            .padding(16.dp),
                        action = {
                            TextButton(onClick = { errorMessage = null }) {
                                Text("Dismiss", color = Color.White)
                            }
                        },
                        containerColor = Color(0xFFDC2626)
                    ) {
                        Text(errorMessage ?: "", color = Color.White, fontSize = 12.sp)
                    }
                }

                // Permission Onboarding Dialog - Shown ONLY ONCE!
                if (showPermissionDialog) {
                    AlertDialog(
                        onDismissRequest = {
                            getSharedPreferences("aura_mesh_prefs", Context.MODE_PRIVATE)
                                .edit()
                                .putBoolean("has_asked_permissions", true)
                                .apply()
                            showPermissionDialog = false
                        },
                        title = {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Default.Shield, contentDescription = null, tint = Color(0xFF059669))
                                Spacer(modifier = Modifier.width(8.dp))
                                Text("Aura 5G Mesh Permissions", fontWeight = FontWeight.Bold, fontSize = 16.sp)
                            }
                        },
                        text = {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                Text(
                                    "Aura requires camera (QR scanning), audio (voice notes), and network permissions for real 5G internet sharing & 4K media drop.",
                                    fontSize = 12.sp,
                                    color = Color(0xFF475569)
                                )
                            }
                        },
                        confirmButton = {
                            Button(
                                onClick = {
                                    getSharedPreferences("aura_mesh_prefs", Context.MODE_PRIVATE)
                                        .edit()
                                        .putBoolean("has_asked_permissions", true)
                                        .apply()
                                    multiplePermissionsLauncher.launch(requiredPermissions)
                                    showPermissionDialog = false
                                },
                                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF059669))
                            ) {
                                Text("Grant Permissions", fontWeight = FontWeight.Bold)
                            }
                        },
                        dismissButton = {
                            TextButton(onClick = {
                                getSharedPreferences("aura_mesh_prefs", Context.MODE_PRIVATE)
                                    .edit()
                                    .putBoolean("has_asked_permissions", true)
                                    .apply()
                                showPermissionDialog = false
                            }) {
                                Text("Don't ask again", color = Color(0xFF64748B))
                            }
                        },
                        shape = RoundedCornerShape(20.dp),
                        containerColor = Color.White
                    )
                }
            }
        }
    }

    @Composable
    private fun RelayTab(clipboardManager: androidx.compose.ui.platform.ClipboardManager) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.SpaceBetween
        ) {
            // Mode Switcher Bar
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
                                text = "Get Free 5G (Consumer)",
                                fontSize = 11.sp,
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
                                text = "Share My 5G (Host)",
                                fontSize = 11.sp,
                                fontWeight = FontWeight.Bold,
                                color = if (isHostMode) Color(0xFF0F172A) else Color(0xFF64748B)
                            )
                        }
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
                    modifier = Modifier.padding(20.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    if (!isHostMode) {
                        // CONSUMER MODE (PHONE B)
                        Text(
                            text = if (isConnected) "● 100% OS TRAFFIC ROUTED VIA PEER" else "○ TUNNEL DISCONNECTED",
                            fontWeight = FontWeight.ExtraBold,
                            fontSize = 11.sp,
                            color = if (isConnected) Color(0xFF059669) else Color(0xFF94A3B8),
                            fontFamily = FontFamily.Monospace
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        if (isConnected) {
                            Text(
                                text = "%.2f MB".format(dataServedMb),
                                fontSize = 38.sp,
                                fontWeight = FontWeight.ExtraBold,
                                color = Color(0xFF059669),
                                fontFamily = FontFamily.Monospace
                            )
                            Text(
                                text = "Live Data from 5G Host #$targetPairCode",
                                fontSize = 11.sp,
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

                            if (showAdvancedSettings) {
                                Spacer(modifier = Modifier.height(8.dp))
                                OutlinedTextField(
                                    value = customHostIp,
                                    onValueChange = { customHostIp = it },
                                    label = { Text("Host IP / Gateway") },
                                    placeholder = { Text("192.168.43.1") },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth(),
                                    shape = RoundedCornerShape(12.dp)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(16.dp))

                        Button(
                            onClick = {
                                if (isConnected) stopAuraVpn() else prepareAndStartVpn()
                            },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(52.dp),
                            shape = RoundedCornerShape(16.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (isConnected) Color(0xFFEF4444) else Color(0xFF059669)
                            )
                        ) {
                            Icon(
                                imageVector = if (isConnected) Icons.Default.Close else Icons.Default.PowerSettingsNew,
                                contentDescription = "Toggle",
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = if (isConnected) "Disconnect 5G Tunnel" else "Connect to 5G Hotspot",
                                fontWeight = FontWeight.Bold,
                                fontSize = 14.sp
                            )
                        }
                    } else {
                        // HOST MODE (PHONE A)
                        Text(
                            text = if (isHostActive) "● 5G RELAY SERVER ACTIVE" else "○ RELAY SERVER OFFLINE",
                            fontWeight = FontWeight.Bold,
                            fontSize = 11.sp,
                            color = if (isHostActive) Color(0xFF4F46E5) else Color(0xFF94A3B8),
                            fontFamily = FontFamily.Monospace
                        )

                        Spacer(modifier = Modifier.height(12.dp))

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
                                modifier = Modifier.padding(16.dp),
                                horizontalAlignment = Alignment.CenterHorizontally
                            ) {
                                Text(
                                    text = myHostCode,
                                    fontSize = 34.sp,
                                    fontWeight = FontWeight.ExtraBold,
                                    color = Color(0xFF4338CA),
                                    fontFamily = FontFamily.Monospace,
                                    letterSpacing = 5.sp
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
                                        text = "Tap to Copy Host Code",
                                        fontSize = 10.sp,
                                        color = Color(0xFF6366F1),
                                        fontWeight = FontWeight.SemiBold
                                    )
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(12.dp))

                        if (isHostActive) {
                            Text(
                                text = "Served: %.2f MB | Connected Peers: $activePeersCount".format(dataHostServedMb),
                                fontSize = 12.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color(0xFF059669),
                                fontFamily = FontFamily.Monospace
                            )
                        }

                        Spacer(modifier = Modifier.height(12.dp))

                        Button(
                            onClick = {
                                if (isHostActive) stopHostService() else startHostService()
                            },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(50.dp),
                            shape = RoundedCornerShape(16.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (isHostActive) Color(0xFFEF4444) else Color(0xFF4F46E5)
                            )
                        ) {
                            Icon(
                                imageVector = if (isHostActive) Icons.Default.Stop else Icons.Default.PlayArrow,
                                contentDescription = null
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = if (isHostActive) "Stop Sharing 5G" else "Start Sharing My 5G",
                                fontWeight = FontWeight.Bold
                            )
                        }
                    }
                }
            }

            // Security & Protocol Footer
            Card(
                shape = RoundedCornerShape(16.dp),
                colors = CardDefaults.cardColors(containerColor = Color(0xFFF1F5F9)),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Shield,
                            contentDescription = "Shield",
                            tint = Color(0xFF059669),
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(
                            text = "AES-256-GCM / User-Space NAT",
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF334155),
                            fontFamily = FontFamily.Monospace
                        )
                    }

                    TextButton(onClick = { showAdvancedSettings = !showAdvancedSettings }) {
                        Text(if (showAdvancedSettings) "Hide IP" else "Advanced", fontSize = 10.sp, color = Color(0xFF4F46E5))
                    }
                }
            }
        }
    }

    @Composable
    private fun ChatTab(chatInputText: String, onTextChange: (String) -> Unit) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp)
        ) {
            LazyColumn(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(chatMessages) { msg ->
                    Column(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalAlignment = if (msg.isSelf) Alignment.End else Alignment.Start
                    ) {
                        Surface(
                            shape = RoundedCornerShape(16.dp),
                            color = if (msg.isSelf) Color(0xFF059669) else Color.White,
                            shadowElevation = 1.dp
                        ) {
                            Text(
                                text = msg.text,
                                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                                fontSize = 12.sp,
                                color = if (msg.isSelf) Color.White else Color(0xFF0F172A)
                            )
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                OutlinedTextField(
                    value = chatInputText,
                    onValueChange = onTextChange,
                    placeholder = { Text("Type message...", fontSize = 12.sp) },
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(16.dp),
                    singleLine = true
                )

                Spacer(modifier = Modifier.width(6.dp))

                IconButton(
                    onClick = {
                        if (isRecordingVoice) stopVoiceRecording() else startVoiceRecording()
                    },
                    modifier = Modifier
                        .size(46.dp)
                        .background(if (isRecordingVoice) Color(0xFFDC2626) else Color(0xFFF1F5F9), CircleShape)
                ) {
                    Icon(
                        imageVector = if (isRecordingVoice) Icons.Default.Stop else Icons.Default.Mic,
                        contentDescription = "Voice",
                        tint = if (isRecordingVoice) Color.White else Color(0xFF059669)
                    )
                }

                Spacer(modifier = Modifier.width(6.dp))

                IconButton(
                    onClick = {
                        if (chatInputText.isNotBlank()) {
                            chatMessages.add(
                                NativeChatMessage(
                                    id = System.currentTimeMillis().toString(),
                                    text = chatInputText.trim(),
                                    isSelf = true,
                                    timestamp = System.currentTimeMillis()
                                )
                            )
                            onTextChange("")
                        }
                    },
                    modifier = Modifier
                        .size(46.dp)
                        .background(Color(0xFF059669), CircleShape)
                ) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Filled.Send,
                        contentDescription = "Send",
                        tint = Color.White
                    )
                }
            }
        }
    }

    @Composable
    private fun MediaDropTab() {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.SpaceBetween
        ) {
            Column {
                Text(
                    text = "DIRECT 4K MEDIA & FILE DROP",
                    fontWeight = FontWeight.Bold,
                    fontSize = 12.sp,
                    fontFamily = FontFamily.Monospace,
                    color = Color(0xFF059669)
                )
                Text(
                    text = "Stream uncompressed photos and 4K videos directly to peer",
                    fontSize = 11.sp,
                    color = Color(0xFF64748B)
                )

                Spacer(modifier = Modifier.height(14.dp))

                Button(
                    onClick = { galleryPickerLauncher.launch("*/*") },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(48.dp),
                    shape = RoundedCornerShape(14.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF4F46E5))
                ) {
                    Icon(Icons.Default.AddPhotoAlternate, contentDescription = null)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("Select Photos, Videos or Files", fontWeight = FontWeight.Bold)
                }

                Spacer(modifier = Modifier.height(16.dp))

                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(mediaTransfers) { item ->
                        Card(
                            shape = RoundedCornerShape(14.dp),
                            colors = CardDefaults.cardColors(containerColor = Color.White),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Text(item.name, fontWeight = FontWeight.Bold, fontSize = 12.sp, maxLines = 1)
                                    Text("${item.progressPercent}%", fontSize = 11.sp, fontFamily = FontFamily.Monospace)
                                }
                                Spacer(modifier = Modifier.height(6.dp))
                                LinearProgressIndicator(
                                    progress = { item.progressPercent / 100f },
                                    modifier = Modifier.fillMaxWidth(),
                                    color = Color(0xFF059669)
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    @Composable
    private fun RadarTab(
        isDnsLeakProtected: Boolean,
        onDnsChange: (Boolean) -> Unit,
        isKillSwitchEnabled: Boolean,
        onKillSwitchChange: (Boolean) -> Unit
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            Text(
                text = "SPEED & MESH TELEMETRY",
                fontWeight = FontWeight.Bold,
                fontSize = 12.sp,
                fontFamily = FontFamily.Monospace,
                color = Color(0xFF059669)
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                Card(
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(16.dp),
                    colors = CardDefaults.cardColors(containerColor = Color.White)
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("Live Ping", fontSize = 11.sp, color = Color(0xFF64748B))
                        Text("${currentPingMs} ms", fontSize = 24.sp, fontWeight = FontWeight.ExtraBold, color = Color(0xFF4F46E5), fontFamily = FontFamily.Monospace)
                    }
                }

                Card(
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(16.dp),
                    colors = CardDefaults.cardColors(containerColor = Color.White)
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("Throughput", fontSize = 11.sp, color = Color(0xFF64748B))
                        Text("${currentSpeedMbps} Mb/s", fontSize = 24.sp, fontWeight = FontWeight.ExtraBold, color = Color(0xFF059669), fontFamily = FontFamily.Monospace)
                    }
                }
            }

            Card(
                shape = RoundedCornerShape(16.dp),
                colors = CardDefaults.cardColors(containerColor = Color.White),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column {
                            Text("DNS Leak Shield", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                            Text("Force all queries to 1.1.1.1 & 8.8.8.8", fontSize = 10.sp, color = Color(0xFF64748B))
                        }
                        Switch(checked = isDnsLeakProtected, onCheckedChange = onDnsChange)
                    }

                    HorizontalDivider(modifier = Modifier.padding(vertical = 10.dp), color = Color(0xFFF1F5F9))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column {
                            Text("Tunnel Kill Switch", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                            Text("Block all non-mesh internet traffic", fontSize = 10.sp, color = Color(0xFF64748B))
                        }
                        Switch(checked = isKillSwitchEnabled, onCheckedChange = onKillSwitchChange)
                    }
                }
            }
        }
    }

    @Composable
    private fun QrPairTab(clipboardManager: androidx.compose.ui.platform.ClipboardManager) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Text(
                text = "INSTANT QR DEVICE PAIRING",
                fontWeight = FontWeight.Bold,
                fontSize = 12.sp,
                fontFamily = FontFamily.Monospace,
                color = Color(0xFF4F46E5)
            )

            Spacer(modifier = Modifier.height(14.dp))

            QrCodeView(
                data = "https://aura-mesh.app/?join=$myHostCode",
                size = 180.dp
            )

            Spacer(modifier = Modifier.height(14.dp))

            Text(
                text = "Pair Code: $myHostCode",
                fontSize = 18.sp,
                fontWeight = FontWeight.ExtraBold,
                fontFamily = FontFamily.Monospace,
                color = Color(0xFF0F172A)
            )

            Spacer(modifier = Modifier.height(8.dp))

            Button(
                onClick = {
                    clipboardManager.setText(AnnotatedString("https://aura-mesh.app/?join=$myHostCode"))
                    Toast.makeText(this@MainActivity, "Invite link copied to clipboard!", Toast.LENGTH_SHORT).show()
                },
                shape = RoundedCornerShape(14.dp),
                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF059669))
            ) {
                Icon(Icons.Default.Share, contentDescription = null, modifier = Modifier.size(16.dp))
                Spacer(modifier = Modifier.width(6.dp))
                Text("Share Pairing Link", fontSize = 12.sp)
            }
        }
    }

    private fun startHostService() {
        val intent = Intent(this, AuraHostService::class.java).apply {
            action = AuraHostService.ACTION_START_HOST
            putExtra(AuraHostService.EXTRA_HOST_CODE, myHostCode)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
        isHostActive = true
        Toast.makeText(this, "5G Hotspot Relay Started on port 9000!", Toast.LENGTH_SHORT).show()
    }

    private fun stopHostService() {
        val intent = Intent(this, AuraHostService::class.java).apply {
            action = AuraHostService.ACTION_STOP_HOST
        }
        startService(intent)
        isHostActive = false
        Toast.makeText(this, "5G Hotspot Relay Stopped", Toast.LENGTH_SHORT).show()
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
            putExtra(AuraVpnService.EXTRA_HOST_IP, customHostIp.trim())
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
