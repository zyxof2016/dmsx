import React from "react";
import { Typography, theme, Tooltip, Button } from "antd";
import { CopyOutlined } from "@ant-design/icons";

const { Text } = Typography;

interface TerminalBlockProps {
  code: string;
  language?: string;
  style?: React.CSSProperties;
}

export const TerminalBlock: React.FC<TerminalBlockProps> = ({ code, style }) => {
  const { token } = theme.useToken();
  const [copied, setCopied] = React.useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      style={{
        position: "relative",
        background: token.colorBgLayout,
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: 6,
        padding: "12px 16px",
        overflowX: "auto",
        ...style,
      }}
    >
      <div style={{ position: "absolute", top: 8, right: 8 }}>
        <Tooltip title={copied ? "已复制" : "复制代码"}>
          <Button
            type="text"
            size="small"
            icon={<CopyOutlined />}
            onClick={handleCopy}
            style={{ color: token.colorTextSecondary }}
          />
        </Tooltip>
      </div>
      <Text
        style={{
          color: token.colorText,
          fontFamily: "var(--ant-font-family-code)",
          fontSize: 13,
          whiteSpace: "pre-wrap",
          wordBreak: "break-all",
          display: "block",
          paddingRight: 32, // space for copy button
        }}
      >
        {code}
      </Text>
    </div>
  );
};
