package com.dmsx.agent;

import android.content.Context;
import android.content.SharedPreferences;

final class AgentConfig {
    private static final String PREFS = "dmsx_agent";

    private AgentConfig() {}

    static SharedPreferences prefs(Context context) {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    static String apiBase(Context context) {
        return trimTrailingSlash(prefs(context).getString("api_base", ""));
    }

    static String tenantId(Context context) {
        return prefs(context).getString("tenant_id", "00000000-0000-0000-0000-000000000001");
    }

    static String enrollmentToken(Context context) {
        return prefs(context).getString("enrollment_token", "");
    }

    static String deviceId(Context context) {
        return prefs(context).getString("device_id", "");
    }

    static String registrationCode(Context context) {
        return prefs(context).getString("registration_code", "");
    }

    static boolean startOnBoot(Context context) {
        return prefs(context).getBoolean("start_on_boot", true);
    }

    static void saveSetup(
            Context context,
            String apiBase,
            String tenantId,
            String enrollmentToken,
            boolean startOnBoot) {
        String oldApiBase = apiBase(context);
        String oldTenantId = tenantId(context);
        String oldToken = enrollmentToken(context);
        String nextApiBase = trimTrailingSlash(apiBase);
        String nextTenantId = tenantId.trim();
        String nextToken = enrollmentToken.trim();

        SharedPreferences.Editor editor = prefs(context)
                .edit()
                .putString("api_base", nextApiBase)
                .putString("tenant_id", nextTenantId)
                .putString("enrollment_token", nextToken)
                .putBoolean("start_on_boot", startOnBoot);
        if (!oldApiBase.equals(nextApiBase) || !oldTenantId.equals(nextTenantId) || !oldToken.equals(nextToken)) {
            editor.remove("device_id").remove("registration_code");
        }
        editor.apply();
    }

    static void saveClaimedDevice(Context context, String deviceId, String registrationCode) {
        prefs(context)
                .edit()
                .putString("device_id", deviceId)
                .putString("registration_code", registrationCode)
                .apply();
    }

    static void saveLastStatus(Context context, String status) {
        prefs(context)
                .edit()
                .putString("last_status", status)
                .putLong("last_status_at", System.currentTimeMillis())
                .apply();
    }

    static String lastStatus(Context context) {
        return prefs(context).getString("last_status", "未启动");
    }

    static long lastStatusAt(Context context) {
        return prefs(context).getLong("last_status_at", 0L);
    }

    static boolean isConfigured(Context context) {
        return !apiBase(context).isEmpty()
                && !tenantId(context).isEmpty()
                && !enrollmentToken(context).isEmpty();
    }

    static String tenantUrl(Context context, String path) {
        return apiBase(context) + "/v1/tenants/" + tenantId(context) + path;
    }

    private static String trimTrailingSlash(String value) {
        String trimmed = value == null ? "" : value.trim();
        while (trimmed.endsWith("/")) {
            trimmed = trimmed.substring(0, trimmed.length() - 1);
        }
        return trimmed;
    }
}
