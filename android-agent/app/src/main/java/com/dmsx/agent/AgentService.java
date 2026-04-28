package com.dmsx.agent;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Intent;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.IBinder;
import android.os.PowerManager;

import org.json.JSONArray;
import org.json.JSONObject;

public class AgentService extends Service {
    static final String ACTION_START = "com.dmsx.agent.START";
    static final String ACTION_STOP = "com.dmsx.agent.STOP";

    private static final String CHANNEL_ID = "dmsx_agent";
    private static final int NOTIFICATION_ID = 1001;
    private static final long HEARTBEAT_MS = 30_000L;
    private static final long COMMAND_POLL_MS = 10_000L;

    private final ApiClient api = new ApiClient();
    private HandlerThread workerThread;
    private Handler worker;
    private PowerManager.WakeLock wakeLock;
    private volatile boolean running;
    private long nextCommandPollAt;

    @Override
    public void onCreate() {
        super.onCreate();
        createNotificationChannel();
        workerThread = new HandlerThread("dmsx-agent-worker");
        workerThread.start();
        worker = new Handler(workerThread.getLooper());
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && ACTION_STOP.equals(intent.getAction())) {
            stopAgent();
            stopSelf();
            return START_NOT_STICKY;
        }
        startForeground(NOTIFICATION_ID, buildNotification("DMSX Agent 运行中"));
        startAgent();
        return START_STICKY;
    }

    @Override
    public void onDestroy() {
        stopAgent();
        if (workerThread != null) {
            workerThread.quitSafely();
        }
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    private void startAgent() {
        if (running) return;
        running = true;
        acquireWakeLock();
        AgentConfig.saveLastStatus(this, "启动中");
        worker.post(this::runLoopOnce);
    }

    private void stopAgent() {
        running = false;
        if (worker != null) worker.removeCallbacksAndMessages(null);
        if (wakeLock != null && wakeLock.isHeld()) wakeLock.release();
        AgentConfig.saveLastStatus(this, "已停止");
    }

    private void runLoopOnce() {
        if (!running) return;
        try {
            if (!AgentConfig.isConfigured(this)) {
                AgentConfig.saveLastStatus(this, "缺少 API、租户或 enrollment token 配置");
                scheduleNext(HEARTBEAT_MS);
                return;
            }
            ensureClaimed();
            sendHeartbeat();
            long now = System.currentTimeMillis();
            if (now >= nextCommandPollAt) {
                pollCommands();
                nextCommandPollAt = now + COMMAND_POLL_MS;
            }
            AgentConfig.saveLastStatus(this, "在线：心跳已上报");
        } catch (Exception error) {
            AgentConfig.saveLastStatus(this, "错误：" + error.getMessage());
        }
        scheduleNext(HEARTBEAT_MS);
    }

    private void scheduleNext(long delayMs) {
        if (running && worker != null) {
            worker.postDelayed(this::runLoopOnce, delayMs);
        }
    }

    private void ensureClaimed() throws Exception {
        if (!AgentConfig.deviceId(this).isEmpty()) return;

        JSONObject body = new JSONObject()
                .put("enrollment_token", AgentConfig.enrollmentToken(this))
                .put("hostname", Telemetry.hostname(this))
                .put("os_version", Build.VERSION.RELEASE)
                .put("agent_version", BuildConfig.VERSION_NAME)
                .put("labels", new JSONObject()
                        .put("agent", "dmsx-android-agent")
                        .put("agent_kind", "android-apk"));
        ApiClient.ApiResponse response = api.post(
                AgentConfig.tenantUrl(this, "/devices/claim-with-enrollment-token"),
                body,
                null);
        if (!response.isSuccess()) {
            throw new IllegalStateException("claim failed: " + response.status + " " + response.body);
        }
        JSONObject device = response.json();
        AgentConfig.saveClaimedDevice(
                this,
                device.optString("id"),
                device.optString("registration_code"));
    }

    private void sendHeartbeat() throws Exception {
        String deviceId = AgentConfig.deviceId(this);
        String token = AgentConfig.enrollmentToken(this);
        api.patch(
                AgentConfig.tenantUrl(this, "/devices/" + deviceId),
                new JSONObject()
                        .put("online_state", "online")
                        .put("agent_version", BuildConfig.VERSION_NAME)
                        .put("os_version", Build.VERSION.RELEASE),
                token);
        ApiClient.ApiResponse response = api.patch(
                AgentConfig.tenantUrl(this, "/devices/" + deviceId + "/shadow/reported"),
                new JSONObject().put("reported", Telemetry.collect(this)),
                token);
        if (!response.isSuccess()) {
            throw new IllegalStateException("shadow update failed: " + response.status + " " + response.body);
        }
    }

    private void pollCommands() throws Exception {
        String deviceId = AgentConfig.deviceId(this);
        ApiClient.ApiResponse response = api.get(
                AgentConfig.tenantUrl(this, "/devices/" + deviceId + "/commands?limit=10"),
                AgentConfig.enrollmentToken(this));
        if (!response.isSuccess()) return;
        JSONArray items = response.json().optJSONArray("items");
        if (items == null) return;
        for (int i = items.length() - 1; i >= 0; i--) {
            JSONObject command = items.optJSONObject(i);
            if (command == null || !"queued".equals(command.optString("status"))) continue;
            executeCommand(command);
        }
    }

    private void executeCommand(JSONObject command) throws Exception {
        String commandId = command.optString("id");
        JSONObject payload = command.optJSONObject("payload");
        String action = payload == null ? "unknown" : payload.optString("action", "unknown");
        String token = AgentConfig.enrollmentToken(this);

        api.patch(
                AgentConfig.tenantUrl(this, "/commands/" + commandId + "/status"),
                new JSONObject().put("status", "running"),
                token);

        int exitCode = 0;
        String stdout;
        String stderr = "";
        if ("smoke_noop".equals(action)) {
            stdout = "smoke_noop ok";
        } else if ("collect_logs".equals(action)) {
            stdout = Telemetry.collect(this).toString();
        } else {
            exitCode = 1;
            stdout = "";
            stderr = "unsupported android action: " + action;
        }

        api.post(
                AgentConfig.tenantUrl(this, "/commands/" + commandId + "/result"),
                new JSONObject()
                        .put("exit_code", exitCode)
                        .put("stdout", stdout)
                        .put("stderr", stderr),
                token);
    }

    private void acquireWakeLock() {
        if (wakeLock != null && wakeLock.isHeld()) return;
        PowerManager pm = (PowerManager) getSystemService(POWER_SERVICE);
        if (pm == null) return;
        wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "dmsx:agent");
        wakeLock.setReferenceCounted(false);
        wakeLock.acquire();
    }

    private Notification buildNotification(String text) {
        Notification.Builder builder = Build.VERSION.SDK_INT >= 26
                ? new Notification.Builder(this, CHANNEL_ID)
                : new Notification.Builder(this);
        return builder
                .setSmallIcon(R.drawable.ic_agent)
                .setContentTitle("DMSX Agent")
                .setContentText(text)
                .setOngoing(true)
                .build();
    }

    private void createNotificationChannel() {
        if (Build.VERSION.SDK_INT < 26) return;
        NotificationChannel channel = new NotificationChannel(
                CHANNEL_ID,
                getString(R.string.agent_channel_name),
                NotificationManager.IMPORTANCE_LOW);
        NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (nm != null) nm.createNotificationChannel(channel);
    }
}
