package com.dmsx.agent;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.NetworkInfo;
import android.os.BatteryManager;
import android.os.Build;
import android.os.SystemClock;
import android.provider.Settings;

import org.json.JSONObject;

import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;
import java.util.TimeZone;

final class Telemetry {
    private Telemetry() {}

    static JSONObject collect(Context context) throws Exception {
        JSONObject android = new JSONObject()
                .put("manufacturer", Build.MANUFACTURER)
                .put("brand", Build.BRAND)
                .put("model", Build.MODEL)
                .put("device", Build.DEVICE)
                .put("product", Build.PRODUCT)
                .put("sdk_version", Build.VERSION.SDK_INT)
                .put("android_version", Build.VERSION.RELEASE)
                .put("security_patch", Build.VERSION.SDK_INT >= 23 ? Build.VERSION.SECURITY_PATCH : "")
                .put("fingerprint", Build.FINGERPRINT)
                .put("supported_abis", join(Build.SUPPORTED_ABIS));

        JSONObject telemetry = new JSONObject()
                .put("agent_kind", "android-apk")
                .put("agent_version", BuildConfig.VERSION_NAME)
                .put("platform", "android")
                .put("hostname", hostname(context))
                .put("os_name", "Android")
                .put("os_version", Build.VERSION.RELEASE)
                .put("uptime_secs", SystemClock.elapsedRealtime() / 1000)
                .put("battery", battery(context))
                .put("network", network(context))
                .put("android", android)
                .put("collected_at", nowIso());
        return telemetry;
    }

    static String hostname(Context context) {
        String model = Build.MODEL == null || Build.MODEL.trim().isEmpty() ? "Android" : Build.MODEL.trim();
        String androidId = Settings.Secure.getString(context.getContentResolver(), Settings.Secure.ANDROID_ID);
        if (androidId == null || androidId.trim().isEmpty()) {
            androidId = Build.DEVICE == null ? "device" : Build.DEVICE;
        }
        return (model + "-" + androidId).replaceAll("[^A-Za-z0-9._-]", "-");
    }

    private static JSONObject battery(Context context) throws Exception {
        BatteryManager bm = (BatteryManager) context.getSystemService(Context.BATTERY_SERVICE);
        int level = bm == null ? -1 : bm.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY);
        boolean charging = false;
        if (bm != null && Build.VERSION.SDK_INT >= 23) {
            charging = bm.isCharging();
        }
        return new JSONObject().put("level_percent", level).put("charging", charging);
    }

    private static JSONObject network(Context context) throws Exception {
        ConnectivityManager cm = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        NetworkInfo info = cm == null ? null : cm.getActiveNetworkInfo();
        return new JSONObject()
                .put("connected", info != null && info.isConnected())
                .put("type", info == null ? "unknown" : info.getTypeName())
                .put("subtype", info == null ? "" : info.getSubtypeName());
    }

    private static String nowIso() {
        SimpleDateFormat fmt = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.US);
        fmt.setTimeZone(TimeZone.getTimeZone("UTC"));
        return fmt.format(new Date());
    }

    private static String join(String[] values) {
        if (values == null || values.length == 0) return "";
        StringBuilder sb = new StringBuilder();
        for (String value : values) {
            if (value == null || value.isEmpty()) continue;
            if (sb.length() > 0) sb.append(',');
            sb.append(value);
        }
        return sb.toString();
    }
}
