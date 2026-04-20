import React from "react";
import { Alert } from "antd";
import { WRITE_DISABLED_REASON } from "../authz";

type Props = {
  visible: boolean;
  resourceLabel?: string;
};

export const ReadonlyBanner: React.FC<Props> = ({ visible, resourceLabel }) => {
  if (!visible) return null;

  return (
    <Alert
      type="info"
      showIcon
      style={{ marginBottom: 16 }}
      message={resourceLabel ? `${resourceLabel}当前为只读模式` : "当前为只读模式"}
      description={WRITE_DISABLED_REASON}
    />
  );
};
