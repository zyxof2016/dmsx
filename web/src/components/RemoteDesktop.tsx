import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Empty,
  Space,
  Spin,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import {
  DesktopOutlined,
  LinkOutlined,
  CopyOutlined,
  InfoCircleOutlined,
  FullscreenOutlined,
  DisconnectOutlined,
  PoweroffOutlined,
  LoadingOutlined,
} from "@ant-design/icons";
import {
  useShadow,
  useCreateDesktopSession,
  useDeleteDesktopSession,
} from "../api/hooks";
import { TENANT_ID } from "../api/client";

const { Text, Paragraph } = Typography;

const RUSTDESK_WEB_CLIENT = "https://web.rustdesk.com";

interface Props {
  deviceId: string;
  deviceHostname?: string;
  devicePlatform?: string;
  deviceOnlineState?: string;
}

type SessionState = "idle" | "connecting" | "connected" | "error";

export const RemoteDesktopPanel: React.FC<Props> = ({
  deviceId,
  deviceHostname,
  devicePlatform,
  deviceOnlineState,
}) => {
  const { data: shadow, isLoading } = useShadow(deviceId);
  const createSession = useCreateDesktopSession();
  const deleteSession = useDeleteDesktopSession();

  const [sessionState, setSessionState] = useState<SessionState>("idle");
  const [errorMsg, setErrorMsg] = useState("");
  const [remoteSize, setRemoteSize] = useState({ width: 0, height: 0 });

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);

  const rustdesk = shadow?.reported?.rustdesk as
    | { installed?: boolean; id?: string; has_permanent_password?: boolean }
    | undefined;
  const rdId = rustdesk?.id;
  const rdInstalled = rustdesk?.installed;
  const hasPwd = rustdesk?.has_permanent_password;
  const isOnline = deviceOnlineState === "online";

  const cleanup = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  }, []);

  useEffect(() => {
    return () => {
      cleanup();
    };
  }, [cleanup]);

  const handleConnect = async () => {
    setSessionState("connecting");
    setErrorMsg("");

    try {
      await createSession.mutateAsync({ deviceId, body: {} });

      const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
      const host = window.location.host;
      const wsUrl = `${proto}//${host}/v1/tenants/${TENANT_ID}/devices/${deviceId}/desktop/ws/viewer`;

      await new Promise<void>((resolve) => setTimeout(resolve, 2000));

      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.binaryType = "arraybuffer";

      ws.onopen = () => {
        setSessionState("connected");
      };

      ws.onmessage = (ev) => {
        if (typeof ev.data === "string") {
          try {
            const msg = JSON.parse(ev.data);
            if (msg.type === "meta") {
              setRemoteSize({ width: msg.width, height: msg.height });
            }
            if (msg.error) {
              setErrorMsg(msg.error);
              setSessionState("error");
            }
          } catch {
            /* ignore non-json text */
          }
          return;
        }

        const blob = new Blob([ev.data], { type: "image/jpeg" });
        const url = URL.createObjectURL(blob);

        if (!imgRef.current) {
          imgRef.current = new Image();
        }
        const img = imgRef.current;

        img.onload = () => {
          const canvas = canvasRef.current;
          if (!canvas) return;
          const ctx = canvas.getContext("2d");
          if (!ctx) return;

          if (canvas.width !== img.width || canvas.height !== img.height) {
            canvas.width = img.width;
            canvas.height = img.height;
          }
          ctx.drawImage(img, 0, 0);
          URL.revokeObjectURL(url);
        };
        img.src = url;
      };

      ws.onerror = () => {
        setErrorMsg("WebSocket connection error");
        setSessionState("error");
      };

      ws.onclose = () => {
        if (sessionState !== "error") {
          setSessionState("idle");
        }
      };
    } catch (e) {
      setErrorMsg(String(e));
      setSessionState("error");
    }
  };

  const handleDisconnect = async () => {
    cleanup();
    try {
      await deleteSession.mutateAsync(deviceId);
    } catch {
      /* best effort */
    }
    setSessionState("idle");
  };

  const mapCoords = (
    e: React.MouseEvent<HTMLCanvasElement>,
  ): { x: number; y: number } => {
    const canvas = canvasRef.current;
    if (!canvas || !remoteSize.width) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    const scaleX = remoteSize.width / rect.width;
    const scaleY = remoteSize.height / rect.height;
    return {
      x: Math.round((e.clientX - rect.left) * scaleX),
      y: Math.round((e.clientY - rect.top) * scaleY),
    };
  };

  const sendInput = (data: Record<string, unknown>) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(data));
    }
  };

  const onMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (sessionState !== "connected") return;
    const { x, y } = mapCoords(e);
    sendInput({ type: "mousemove", x, y });
  };

  const onMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    const { x, y } = mapCoords(e);
    const button =
      e.button === 2 ? "right" : e.button === 1 ? "middle" : "left";
    sendInput({ type: "mousedown", button, x, y });
  };

  const onMouseUp = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    const { x, y } = mapCoords(e);
    const button =
      e.button === 2 ? "right" : e.button === 1 ? "middle" : "left";
    sendInput({ type: "mouseup", button, x, y });
  };

  const onWheel = (e: React.WheelEvent<HTMLCanvasElement>) => {
    if (sessionState !== "connected") return;
    const { x, y } = mapCoords(e as unknown as React.MouseEvent<HTMLCanvasElement>);
    sendInput({ type: "scroll", x, y, deltaX: e.deltaX, deltaY: e.deltaY });
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLCanvasElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    sendInput({
      type: "keydown",
      key: e.key,
      code: e.code,
      modifiers: [
        e.ctrlKey && "ctrl",
        e.shiftKey && "shift",
        e.altKey && "alt",
        e.metaKey && "meta",
      ].filter(Boolean),
    });
  };

  const onKeyUp = (e: React.KeyboardEvent<HTMLCanvasElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    sendInput({ type: "keyup", key: e.key, code: e.code });
  };

  const onCtrlAltDel = () => {
    sendInput({
      type: "keydown",
      key: "Delete",
      code: "Delete",
      modifiers: ["ctrl", "alt"],
    });
    setTimeout(() => {
      sendInput({ type: "keyup", key: "Delete", code: "Delete" });
    }, 100);
  };

  const onFullscreen = () => {
    containerRef.current?.requestFullscreen?.();
  };

  const onContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
  };

  const openRustDeskWeb = () => {
    if (!rdId) return;
    window.open(
      `${RUSTDESK_WEB_CLIENT}/#/connection/${rdId}`,
      "_blank",
      "noopener,noreferrer",
    );
  };

  const openRustDeskNative = () => {
    if (!rdId) return;
    window.open(`rustdesk://connection/new/${rdId}`, "_self");
  };

  const copyId = () => {
    if (rdId) navigator.clipboard.writeText(rdId);
  };

  if (isLoading) return <Spin />;

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="middle">
      {/* --- WebRTC / WebSocket Remote Desktop --- */}
      <Card
        title="远程桌面 (WebRTC)"
        size="small"
        extra={
          sessionState === "connected" ? (
            <Space>
              <Tag color="green">已连接</Tag>
              {remoteSize.width > 0 && (
                <Tag>{remoteSize.width}x{remoteSize.height}</Tag>
              )}
            </Space>
          ) : sessionState === "connecting" ? (
            <Tag icon={<LoadingOutlined />} color="processing">
              连接中...
            </Tag>
          ) : null
        }
      >
        {sessionState === "idle" && (
          <Space direction="vertical" style={{ width: "100%" }}>
            {!isOnline && (
              <Alert
                type="warning"
                message="设备当前离线，远程桌面可能无法建立"
                showIcon
              />
            )}
            <Button
              type="primary"
              size="large"
              icon={<DesktopOutlined />}
              onClick={handleConnect}
              loading={createSession.isPending}
            >
              连接远程桌面
            </Button>
            <Alert
              type="info"
              showIcon
              message="连接说明"
              description="点击后系统将向设备 Agent 下发屏幕采集指令，Agent 采集屏幕画面并实时回传。您可以直接在浏览器内查看远端画面并进行键鼠控制。"
            />
          </Space>
        )}

        {sessionState === "error" && (
          <Space direction="vertical" style={{ width: "100%" }}>
            <Alert type="error" message="连接失败" description={errorMsg} showIcon />
            <Button onClick={() => setSessionState("idle")}>重试</Button>
          </Space>
        )}

        {(sessionState === "connecting" || sessionState === "connected") && (
          <div ref={containerRef} style={{ position: "relative" }}>
            {/* Toolbar */}
            <Space
              style={{
                marginBottom: 8,
                padding: "4px 8px",
                background: "#f5f5f5",
                borderRadius: 4,
              }}
            >
              <Tooltip title="全屏">
                <Button
                  size="small"
                  icon={<FullscreenOutlined />}
                  onClick={onFullscreen}
                />
              </Tooltip>
              <Tooltip title="Ctrl+Alt+Del">
                <Button size="small" icon={<PoweroffOutlined />} onClick={onCtrlAltDel}>
                  Ctrl+Alt+Del
                </Button>
              </Tooltip>
              <Tooltip title="断开连接">
                <Button
                  size="small"
                  danger
                  icon={<DisconnectOutlined />}
                  onClick={handleDisconnect}
                >
                  断开
                </Button>
              </Tooltip>
            </Space>

            {sessionState === "connecting" && (
              <div
                style={{
                  textAlign: "center",
                  padding: 48,
                  background: "#000",
                  borderRadius: 4,
                }}
              >
                <Spin
                  size="large"
                  tip="等待 Agent 开始屏幕采集..."
                  style={{ color: "#fff" }}
                >
                  <div style={{ height: 80 }} />
                </Spin>
              </div>
            )}

            <canvas
              ref={canvasRef}
              tabIndex={0}
              style={{
                display: sessionState === "connected" ? "block" : "none",
                width: "100%",
                background: "#000",
                borderRadius: 4,
                cursor: "default",
                outline: "none",
              }}
              onMouseMove={onMouseMove}
              onMouseDown={onMouseDown}
              onMouseUp={onMouseUp}
              onWheel={onWheel}
              onKeyDown={onKeyDown}
              onKeyUp={onKeyUp}
              onContextMenu={onContextMenu}
            />
          </div>
        )}
      </Card>

      {/* --- RustDesk Fallback --- */}
      <Card title="RustDesk 远程桌面 (备选)" size="small">
        {!rdInstalled || !rdId ? (
          <Empty
            image={
              <DesktopOutlined style={{ fontSize: 36, color: "#bfbfbf" }} />
            }
            description="该设备未安装 RustDesk"
          >
            <Alert
              type="info"
              showIcon
              icon={<InfoCircleOutlined />}
              message="如何启用 RustDesk 远程桌面"
              description={
                <div>
                  <Paragraph>
                    1. 在目标设备上安装{" "}
                    <a
                      href="https://rustdesk.com"
                      target="_blank"
                      rel="noreferrer"
                    >
                      RustDesk
                    </a>
                  </Paragraph>
                  <Paragraph>2. 设置永久密码</Paragraph>
                  <Paragraph>
                    3. 配置自建中继服务器并重启 Agent
                  </Paragraph>
                </div>
              }
            />
          </Empty>
        ) : (
          <Space direction="vertical" style={{ width: "100%" }} size="small">
            <Descriptions column={2} size="small">
              <Descriptions.Item label="RustDesk ID">
                <Space>
                  <Text code style={{ fontWeight: 700 }}>
                    {rdId}
                  </Text>
                  <Button
                    size="small"
                    icon={<CopyOutlined />}
                    onClick={copyId}
                  />
                </Space>
              </Descriptions.Item>
              <Descriptions.Item label="永久密码">
                {hasPwd ? (
                  <Tag color="green">已设置</Tag>
                ) : (
                  <Tag color="orange">未设置</Tag>
                )}
              </Descriptions.Item>
            </Descriptions>
            <Space>
              <Button
                type="primary"
                icon={<DesktopOutlined />}
                onClick={openRustDeskWeb}
                disabled={!rdId}
              >
                浏览器连接
              </Button>
              <Button
                icon={<LinkOutlined />}
                onClick={openRustDeskNative}
                disabled={!rdId}
              >
                本地客户端
              </Button>
            </Space>
          </Space>
        )}
      </Card>

      {/* Device info */}
      <Card title="设备信息" size="small">
        <Descriptions column={2} size="small">
          <Descriptions.Item label="主机名">
            {deviceHostname ?? "—"}
          </Descriptions.Item>
          <Descriptions.Item label="平台">
            {devicePlatform ?? "—"}
          </Descriptions.Item>
          <Descriptions.Item label="状态">
            {isOnline ? (
              <Tag color="green">在线</Tag>
            ) : (
              <Tag color="red">离线</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Device ID">
            <Text code copyable={{ text: deviceId }}>
              {deviceId.slice(0, 8)}...
            </Text>
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  );
};
