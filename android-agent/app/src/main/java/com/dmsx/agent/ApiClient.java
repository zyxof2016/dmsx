package com.dmsx.agent;

import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

final class ApiClient {
    static final class ApiResponse {
        final int status;
        final String body;

        ApiResponse(int status, String body) {
            this.status = status;
            this.body = body;
        }

        boolean isSuccess() {
            return status >= 200 && status < 300;
        }

        JSONObject json() throws Exception {
            return new JSONObject(body == null || body.isEmpty() ? "{}" : body);
        }
    }

    ApiResponse get(String url, String deviceToken) throws IOException {
        return request("GET", url, null, deviceToken);
    }

    ApiResponse post(String url, JSONObject body, String deviceToken) throws IOException {
        return request("POST", url, body, deviceToken);
    }

    ApiResponse patch(String url, JSONObject body, String deviceToken) throws IOException {
        return request("PATCH", url, body, deviceToken);
    }

    private ApiResponse request(String method, String url, JSONObject body, String deviceToken) throws IOException {
        HttpURLConnection conn = (HttpURLConnection) new URL(url).openConnection();
        conn.setRequestMethod(method);
        conn.setConnectTimeout(15_000);
        conn.setReadTimeout(20_000);
        conn.setRequestProperty("Accept", "application/json");
        conn.setRequestProperty("Content-Type", "application/json; charset=utf-8");
        if (deviceToken != null && !deviceToken.trim().isEmpty()) {
            conn.setRequestProperty("X-DMSX-Device-Token", deviceToken.trim());
        }
        if (body != null) {
            byte[] raw = body.toString().getBytes(StandardCharsets.UTF_8);
            conn.setDoOutput(true);
            conn.setFixedLengthStreamingMode(raw.length);
            try (OutputStream out = conn.getOutputStream()) {
                out.write(raw);
            }
        }

        int status = conn.getResponseCode();
        InputStream stream = status >= 400 ? conn.getErrorStream() : conn.getInputStream();
        String responseBody = stream == null ? "" : readAll(stream);
        conn.disconnect();
        return new ApiResponse(status, responseBody);
    }

    private static String readAll(InputStream stream) throws IOException {
        StringBuilder sb = new StringBuilder();
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(stream, StandardCharsets.UTF_8))) {
            String line;
            while ((line = reader.readLine()) != null) {
                sb.append(line);
            }
        }
        return sb.toString();
    }
}
