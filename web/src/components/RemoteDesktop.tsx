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
import type { DesktopSessionResponse } from "../api/types";

type LiveKitModule = typeof import("livekit-client");
type LiveKitRoom = import("livekit-client").Room;

const { Text, Paragraph } = Typography;

const RUSTDESK_WEB_CLIENT = "https://web.rustdesk.com";

interface Props {
  deviceId: string;
  deviceHostname?: string;
  devicePlatform?: string;
  deviceOnlineState?: string;
}

type SessionState =
  | "idle"
  | "creating"
  | "waiting_agent"
  | "connected"
  | "reconnecting"
  | "disconnected"
  | "error";

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
  const [hasFocus, setHasFocus] = useState(false);

  const stageRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const roomRef = useRef<LiveKitRoom | null>(null);
  const sessionRef = useRef<DesktopSessionResponse | null>(null);
  const manualDisconnectRef = useRef(false);
  const liveKitModuleRef = useRef<LiveKitModule | null>(null);

  const rustdesk = shadow?.reported?.rustdesk as
    | { installed?: boolean; id?: string; has_permanent_password?: boolean }
    | undefined;
  const rdId = rustdesk?.id;
  const rdInstalled = rustdesk?.installed;
  const hasPwd = rustdesk?.has_permanent_password;
  const isOnline = deviceOnlineState === "online";

  const detachVideo = useCallback(() => {
    if (videoRef.current) {
      videoRef.current.srcObject = null;
    }
  }, []);

  const disconnectRoom = useCallback(async () => {
    const room = roomRef.current;
    roomRef.current = null;
    if (room) {
      room.removeAllListeners();
      await room.disconnect();
    }
    detachVideo();
  }, [detachVideo]);

  const destroySession = useCallback(async () => {
    const session = sessionRef.current;
    sessionRef.current = null;
    if (!session) return;
    try {
      await deleteSession.mutateAsync({
        deviceId,
        sessionId: session.session_id,
      });
    } catch {
      /* best effort */
    }
  }, [deleteSession, deviceId]);

  const fullCleanup = useCallback(
    async (destroyRemote: boolean) => {
      await disconnectRoom();
      if (destroyRemote) {
        await destroySession();
      }
      setRemoteSize({ width: 0, height: 0 });
      setHasFocus(false);
    },
    [destroySession, disconnectRoom],
  );

  useEffect(() => {
    return () => {
      void fullCleanup(true);
    };
  }, [fullCleanup]);

  const loadLiveKit = useCallback(async () => {
    if (!liveKitModuleRef.current) {
      liveKitModuleRef.current = await import("livekit-client");
    }
    return liveKitModuleRef.current;
  }, []);

  const attachTrack = useCallback((room: LiveKitRoom) => {
    let attached = false;
    room.remoteParticipants.forEach((participant) => {
      participant.trackPublications.forEach((publication) => {
        const subscribedTrack = publication.track;
        if (
          subscribedTrack &&
          subscribedTrack.kind === "video" &&
          videoRef.current
        ) {
          subscribedTrack.attach(videoRef.current);
          attached = true;
        }
      });
    });

    if (attached) {
      setSessionState("connected");
      requestAnimationFrame(() => stageRef.current?.focus());
    }
  }, []);

  const wireRoom = useCallback(
    (room: LiveKitRoom, livekit: LiveKitModule) => {
      room
        .on(livekit.RoomEvent.ConnectionStateChanged, (state) => {
          if (state === "reconnecting") {
            setSessionState("reconnecting");
          } else if (state === "connected") {
            setSessionState("waiting_agent");
            attachTrack(room);
          }
        })
        .on(livekit.RoomEvent.TrackSubscribed, (track) => {
          if (track.kind === "video" && videoRef.current) {
            track.attach(videoRef.current);
            setSessionState("connected");
            requestAnimationFrame(() => stageRef.current?.focus());
          }
        })
        .on(livekit.RoomEvent.TrackUnsubscribed, (track) => {
          if (videoRef.current) {
            track.detach(videoRef.current);
          }
          detachVideo();
          setSessionState("waiting_agent");
        })
        .on(livekit.RoomEvent.Disconnected, () => {
          detachVideo();
          setHasFocus(false);
          setSessionState(
            manualDisconnectRef.current ? "disconnected" : "reconnecting",
          );
        });
    },
    [attachTrack, detachVideo],
  );

  const connectToRoom = useCallback(
    async (session: DesktopSessionResponse) => {
      const livekit = await loadLiveKit();
      const room = new livekit.Room({
        adaptiveStream: true,
        dynacast: true,
      });
      wireRoom(room, livekit);
      roomRef.current = room;
      await room.connect(session.livekit_url, session.token);
      attachTrack(room);
    },
    [attachTrack, loadLiveKit, wireRoom],
  );

  const handleConnect = async () => {
    manualDisconnectRef.current = false;
    setSessionState("creating");
    setErrorMsg("");

    try {
      await fullCleanup(true);
      const session = await createSession.mutateAsync({ deviceId, body: {} });
      sessionRef.current = session;
      setSessionState("waiting_agent");
      await connectToRoom(session);
    } catch (e) {
      setErrorMsg(String(e));
      setSessionState("error");
    }
  };

  const handleDisconnect = async () => {
    manualDisconnectRef.current = true;
    await fullCleanup(true);
    setSessionState("idle");
  };

  const mapCoords = (
    e: React.MouseEvent<HTMLDivElement>,
  ): { x: number; y: number } => {
    const surface = stageRef.current;
    if (!surface || !remoteSize.width) return { x: 0, y: 0 };
    const rect = surface.getBoundingClientRect();
    const scaleX = remoteSize.width / rect.width;
    const scaleY = remoteSize.height / rect.height;
    return {
      x: Math.round((e.clientX - rect.left) * scaleX),
      y: Math.round((e.clientY - rect.top) * scaleY),
    };
  };

  const sendInput = useCallback(
    async (data: Record<string, unknown>, reliable = false) => {
      const room = roomRef.current;
      if (!room || room.state !== "connected") return;
      const payload = new TextEncoder().encode(JSON.stringify(data));
      await room.localParticipant.publishData(payload, {
        reliable,
        topic: "desktop-input",
      });
    },
    [],
  );

  const onMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (sessionState !== "connected") return;
    const { x, y } = mapCoords(e);
    void sendInput({ type: "mousemove", x, y });
  };

  const onMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    stageRef.current?.focus();
    setHasFocus(true);
    const { x, y } = mapCoords(e);
    const button =
      e.button === 2 ? "right" : e.button === 1 ? "middle" : "left";
    void sendInput({ type: "mousedown", button, x, y }, true);
  };

  const onMouseUp = (e: React.MouseEvent<HTMLDivElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    const { x, y } = mapCoords(e);
    const button =
      e.button === 2 ? "right" : e.button === 1 ? "middle" : "left";
    void sendInput({ type: "mouseup", button, x, y }, true);
  };

  const onWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    const { x, y } = mapCoords(e as unknown as React.MouseEvent<HTMLDivElement>);
    void sendInput(
      { type: "scroll", x, y, deltaX: e.deltaX, deltaY: e.deltaY },
      false,
    );
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    void sendInput(
      {
      type: "keydown",
      key: e.key,
      code: e.code,
      modifiers: [
        e.ctrlKey && "ctrl",
        e.shiftKey && "shift",
        e.altKey && "alt",
        e.metaKey && "meta",
      ].filter(Boolean),
      },
      true,
    );
  };

  const onKeyUp = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (sessionState !== "connected") return;
    e.preventDefault();
    void sendInput(
      {
        type: "keyup",
        key: e.key,
        code: e.code,
        modifiers: [
          e.ctrlKey && "ctrl",
          e.shiftKey && "shift",
          e.altKey && "alt",
          e.metaKey && "meta",
        ].filter(Boolean),
      },
      true,
    );
  };

  const onCtrlAltDel = () => {
    void sendInput(
      {
        type: "keydown",
        key: "Delete",
        code: "Delete",
        modifiers: ["ctrl", "alt"],
      },
      true,
    );
    setTimeout(() => {
      void sendInput(
        {
          type: "keyup",
          key: "Delete",
          code: "Delete",
          modifiers: ["ctrl", "alt"],
        },
        true,
      );
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
        title="远程桌面 (LiveKit WebRTC)"
        size="small"
        extra={
          sessionState === "connected" ? (
            <Space>
              <Tag color="green">已连接</Tag>
              {remoteSize.width > 0 && (
                <Tag>{remoteSize.width}x{remoteSize.height}</Tag>
              )}
              {hasFocus ? <Tag color="blue">键盘已接管</Tag> : null}
            </Space>
          ) : ["creating", "waiting_agent", "reconnecting"].includes(
              sessionState,
            ) ? (
            <Tag icon={<LoadingOutlined />} color="processing">
              {sessionState === "creating"
                ? "创建会话中..."
                : sessionState === "reconnecting"
                  ? "重连中..."
                  : "等待设备接入..."}
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
              description="点击后系统会创建 LiveKit 房间并向设备 Agent 下发入会指令。Agent 发布屏幕轨后，浏览器会直接订阅视频，并通过 Data Channel 下发键鼠事件。"
            />
          </Space>
        )}

        {sessionState === "error" && (
          <Space direction="vertical" style={{ width: "100%" }}>
            <Alert type="error" message="连接失败" description={errorMsg} showIcon />
            <Button
              onClick={() => {
                setErrorMsg("");
                setSessionState("idle");
              }}
            >
              重试
            </Button>
          </Space>
        )}

        {sessionState === "disconnected" && (
          <Space direction="vertical" style={{ width: "100%" }}>
            <Alert
              type="info"
              showIcon
              message="会话已断开"
              description="远程桌面已显式关闭，重新连接会创建新的桌面会话。"
            />
            <Button type="primary" onClick={handleConnect}>
              重新连接
            </Button>
          </Space>
        )}

        {["creating", "waiting_agent", "connected", "reconnecting"].includes(
          sessionState,
        ) && (
          <div ref={containerRef} style={{ position: "relative" }}>
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

            {sessionState !== "connected" && (
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
                  tip={
                    sessionState === "creating"
                      ? "正在创建桌面会话..."
                      : sessionState === "reconnecting"
                        ? "网络恢复中，正在重连房间..."
                        : "已进房，等待设备发布屏幕轨..."
                  }
                  style={{ color: "#fff" }}
                >
                  <div style={{ height: 80 }} />
                </Spin>
              </div>
            )}

            {sessionState === "connected" && !hasFocus && (
              <Alert
                type="info"
                showIcon
                style={{ marginBottom: 8 }}
                message="已连接，点击画面接管键盘"
              />
            )}

            <div
              ref={stageRef}
              tabIndex={0}
              style={{
                display: sessionState === "connected" ? "block" : "none",
                width: "100%",
                minHeight: 360,
                background: "#000",
                borderRadius: 4,
                cursor: "default",
                outline: "none",
                overflow: "hidden",
              }}
              onMouseMove={onMouseMove}
              onMouseDown={onMouseDown}
              onMouseUp={onMouseUp}
              onWheel={onWheel}
              onKeyDown={onKeyDown}
              onKeyUp={onKeyUp}
              onContextMenu={onContextMenu}
              onFocus={() => setHasFocus(true)}
              onBlur={() => setHasFocus(false)}
            >
              <video
                ref={videoRef}
                autoPlay
                playsInline
                muted
                style={{ display: "block", width: "100%" }}
                onLoadedMetadata={(e) => {
                  const video = e.currentTarget;
                  setRemoteSize({
                    width: video.videoWidth,
                    height: video.videoHeight,
                  });
                }}
              />
            </div>
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
