import React from "react";
import { Button, Tooltip } from "antd";
import type { ButtonProps } from "antd";
import { WRITE_DISABLED_REASON } from "../authz";

type Props = ButtonProps & {
  allowed?: boolean;
  disabledReason?: React.ReactNode;
};

export const GuardedButton: React.FC<Props> = ({
  allowed = true,
  disabledReason = WRITE_DISABLED_REASON,
  children,
  disabled,
  ...rest
}) => {
  const finalDisabled = disabled || !allowed;
  const button = (
    <Button {...rest} disabled={finalDisabled}>
      {children}
    </Button>
  );

  if (allowed || !finalDisabled) {
    return button;
  }

  return <Tooltip title={disabledReason}>{button}</Tooltip>;
};
