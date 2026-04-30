package com.dmsx.agent;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

public class MainActivity extends Activity {
    private EditText apiBaseInput;
    private EditText tenantIdInput;
    private EditText enrollmentTokenInput;
    private CheckBox startOnBootInput;
    private TextView statusText;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        maybeRequestNotificationPermission();
        boolean loadedBundledSetup = AgentConfig.applyBundledSetupIfNeeded(this);
        setContentView(buildContentView());
        loadConfig();
        if (loadedBundledSetup || (AgentConfig.hasBundledSetup() && AgentConfig.isConfigured(this))) {
            startAgentService();
        }
        refreshStatus();
    }

    @Override
    protected void onResume() {
        super.onResume();
        refreshStatus();
    }

    private View buildContentView() {
        ScrollView scroll = new ScrollView(this);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(20), dp(24), dp(20), dp(24));
        scroll.addView(root);

        TextView title = new TextView(this);
        title.setText("DMSX Android Agent");
        title.setTextSize(24);
        title.setTextColor(0xFF111827);
        root.addView(title);

        TextView subtitle = new TextView(this);
        subtitle.setText("安装包可内置注册配置；首次打开后会自动启动前台服务。也可以手动填写 API URL、Tenant ID 和 enrollment token。\n");
        subtitle.setTextColor(0xFF4B5563);
        root.addView(subtitle);

        apiBaseInput = input("API URL，例如 http://192.168.1.10:8080");
        root.addView(label("API URL"));
        root.addView(apiBaseInput);

        tenantIdInput = input("Tenant ID");
        root.addView(label("Tenant ID"));
        root.addView(tenantIdInput);

        enrollmentTokenInput = input("Enrollment Token");
        enrollmentTokenInput.setMinLines(3);
        enrollmentTokenInput.setSingleLine(false);
        root.addView(label("Enrollment Token"));
        root.addView(enrollmentTokenInput);

        startOnBootInput = new CheckBox(this);
        startOnBootInput.setText("开机后自动启动 Agent");
        root.addView(startOnBootInput);

        Button save = button("保存配置");
        save.setOnClickListener(v -> saveConfig());
        root.addView(save);

        Button start = button("启动 Agent");
        start.setOnClickListener(v -> {
            saveConfig();
            startAgentService();
            refreshStatus();
        });
        root.addView(start);

        Button stop = button("停止 Agent");
        stop.setOnClickListener(v -> {
            stopAgentService();
            refreshStatus();
        });
        root.addView(stop);

        statusText = new TextView(this);
        statusText.setTextColor(0xFF111827);
        statusText.setPadding(0, dp(16), 0, 0);
        root.addView(statusText);

        return scroll;
    }

    private void loadConfig() {
        apiBaseInput.setText(AgentConfig.apiBase(this));
        tenantIdInput.setText(AgentConfig.tenantId(this));
        enrollmentTokenInput.setText(AgentConfig.enrollmentToken(this));
        startOnBootInput.setChecked(AgentConfig.startOnBoot(this));
    }

    private void saveConfig() {
        String apiBase = apiBaseInput.getText().toString().trim();
        String tenantId = tenantIdInput.getText().toString().trim();
        String token = enrollmentTokenInput.getText().toString().trim();
        if (apiBase.isEmpty() || tenantId.isEmpty() || token.isEmpty()) {
            Toast.makeText(this, "API URL、Tenant ID 和 token 都不能为空", Toast.LENGTH_LONG).show();
            return;
        }
        AgentConfig.saveSetup(this, apiBase, tenantId, token, startOnBootInput.isChecked());
        Toast.makeText(this, "配置已保存", Toast.LENGTH_SHORT).show();
        refreshStatus();
    }

    private void startAgentService() {
        Intent intent = new Intent(this, AgentService.class).setAction(AgentService.ACTION_START);
        if (Build.VERSION.SDK_INT >= 26) {
            startForegroundService(intent);
        } else {
            startService(intent);
        }
    }

    private void stopAgentService() {
        Intent intent = new Intent(this, AgentService.class).setAction(AgentService.ACTION_STOP);
        startService(intent);
    }

    private void refreshStatus() {
        String deviceId = AgentConfig.deviceId(this);
        String registrationCode = AgentConfig.registrationCode(this);
        long at = AgentConfig.lastStatusAt(this);
        statusText.setText(
                "状态：" + AgentConfig.lastStatus(this)
                        + "\n设备 ID：" + (deviceId.isEmpty() ? "未认领" : deviceId)
                        + "\n注册码：" + (registrationCode.isEmpty() ? "未认领" : registrationCode)
                        + "\n更新时间：" + (at == 0 ? "无" : new java.util.Date(at).toString()));
    }

    private TextView label(String value) {
        TextView label = new TextView(this);
        label.setText(value);
        label.setTextColor(0xFF374151);
        label.setPadding(0, dp(12), 0, dp(4));
        return label;
    }

    private EditText input(String hint) {
        EditText input = new EditText(this);
        input.setHint(hint);
        input.setSingleLine(true);
        input.setTextSize(14);
        return input;
    }

    private Button button(String text) {
        Button button = new Button(this);
        button.setText(text);
        return button;
    }

    private int dp(int value) {
        return (int) (value * getResources().getDisplayMetrics().density + 0.5f);
    }

    private void maybeRequestNotificationPermission() {
        if (Build.VERSION.SDK_INT >= 33
                && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS}, 10);
        }
    }
}
