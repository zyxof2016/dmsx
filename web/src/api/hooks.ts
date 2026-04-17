import {
  useQuery,
  useMutation,
  useQueryClient,
  keepPreviousData,
} from "@tanstack/react-query";
import { api, tenantPath, buildQuery } from "./client";
import type {
  Device,
  Policy,
  Command,
  Artifact,
  ComplianceFinding,
  DashboardStats,
  ListResponse,
  ListParams,
  CreateDeviceReq,
  CreatePolicyReq,
  PolicyEditorPublishReq,
  PolicyRevision,
  CreateCommandReq,
  CreateArtifactReq,
  ShadowResponse,
  UpdateShadowDesiredReq,
  DeviceActionReq,
  CommandResult,
  SubmitCommandResultReq,
  UpdateCommandStatusReq,
  DesktopSessionResponse,
  DesktopSessionCreateReq,
  LivekitConfigResponse,
  AuditLog,
  AuditLogListParams,
  SystemSetting,
  SystemSettingUpsertReq,
  RbacRole,
} from "./types";

// ---- Dashboard ----

export function useStats() {
  return useQuery({
    queryKey: ["stats"],
    queryFn: () => api.get<DashboardStats>(tenantPath("/stats")),
    refetchInterval: 15_000,
  });
}

// ---- Devices ----

export function useDevices(params?: ListParams) {
  return useQuery({
    queryKey: ["devices", params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Device>>(
        tenantPath(`/devices${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
    refetchInterval: 10_000,
  });
}

export function useDevice(id: string | undefined) {
  return useQuery({
    queryKey: ["device", id],
    queryFn: () => api.get<Device>(tenantPath(`/devices/${id}`)),
    enabled: !!id,
  });
}

export function useCreateDevice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateDeviceReq) =>
      api.post<Device>(tenantPath("/devices"), data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["devices"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

export function useDeleteDevice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.del(tenantPath(`/devices/${id}`)),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["devices"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

// ---- Policies ----

export function usePolicies(params?: ListParams) {
  return useQuery({
    queryKey: ["policies", params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Policy>>(
        tenantPath(`/policies${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
  });
}

export function usePolicy(id: string | undefined) {
  return useQuery({
    queryKey: ["policy", id],
    queryFn: () => api.get<Policy>(tenantPath(`/policies/${id}`)),
    enabled: !!id,
  });
}

export function useCreatePolicy() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreatePolicyReq) =>
      api.post<Policy>(tenantPath("/policies"), data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["policies"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

export function useDeletePolicy() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.del(tenantPath(`/policies/${id}`)),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["policies"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

// ---- Commands ----

export function useCommands(params?: ListParams) {
  return useQuery({
    queryKey: ["commands", params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Command>>(
        tenantPath(`/commands${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
    refetchInterval: 10_000,
  });
}

export function useCommand(id: string | undefined) {
  return useQuery({
    queryKey: ["command", id],
    queryFn: () => api.get<Command>(tenantPath(`/commands/${id}`)),
    enabled: !!id,
    refetchInterval: (query) => {
      const command = query.state.data as Command | undefined;
      if (!command) return false;
      return ["queued", "delivered", "acked", "running"].includes(command.status)
        ? 3_000
        : false;
    },
  });
}

export function useCreateCommand() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateCommandReq) =>
      api.post<Command>(tenantPath("/commands"), data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["commands"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

// ---- Device Shadow ----

export function useShadow(deviceId: string | undefined) {
  return useQuery({
    queryKey: ["shadow", deviceId],
    queryFn: () =>
      api.get<ShadowResponse>(tenantPath(`/devices/${deviceId}/shadow`)),
    enabled: !!deviceId,
    refetchInterval: 10_000,
  });
}

export function useUpdateShadowDesired() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      deviceId,
      desired,
    }: {
      deviceId: string;
      desired: UpdateShadowDesiredReq;
    }) =>
      api.patch<ShadowResponse>(
        tenantPath(`/devices/${deviceId}/shadow/desired`),
        desired,
      ),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["shadow", vars.deviceId] });
    },
  });
}

// ---- Device Actions (remote control) ----

export function useDeviceAction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      deviceId,
      action,
    }: {
      deviceId: string;
      action: DeviceActionReq;
    }) =>
      api.post<Command>(tenantPath(`/devices/${deviceId}/actions`), action),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["deviceCommands", vars.deviceId] });
      qc.invalidateQueries({ queryKey: ["commands"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

export function useDeviceCommands(
  deviceId: string | undefined,
  params?: ListParams,
) {
  return useQuery({
    queryKey: ["deviceCommands", deviceId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Command>>(
        tenantPath(`/devices/${deviceId}/commands${buildQuery(params)}`),
      ),
    enabled: !!deviceId,
    placeholderData: keepPreviousData,
    refetchInterval: 10_000,
  });
}

// ---- Command Result ----

export function useCommandResult(commandId: string | undefined) {
  return useQuery({
    queryKey: ["commandResult", commandId],
    queryFn: () =>
      api.get<CommandResult>(tenantPath(`/commands/${commandId}/result`)),
    enabled: !!commandId,
    retry: false,
    refetchInterval: (query) =>
      commandId && !query.state.data ? 3_000 : false,
  });
}

export function useSubmitCommandResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      commandId,
      body,
    }: {
      commandId: string;
      body: SubmitCommandResultReq;
    }) =>
      api.post<CommandResult>(
        tenantPath(`/commands/${commandId}/result`),
        body,
      ),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["commandResult", vars.commandId] });
      qc.invalidateQueries({ queryKey: ["command", vars.commandId] });
      qc.invalidateQueries({ queryKey: ["commands"] });
    },
  });
}

export function useUpdateCommandStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      commandId,
      body,
    }: {
      commandId: string;
      body: UpdateCommandStatusReq;
    }) =>
      api.patch<Command>(tenantPath(`/commands/${commandId}/status`), body),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["command", vars.commandId] });
      qc.invalidateQueries({ queryKey: ["commands"] });
    },
  });
}

// ---- Artifacts ----

export function useArtifacts(params?: ListParams) {
  return useQuery({
    queryKey: ["artifacts", params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Artifact>>(
        tenantPath(`/artifacts${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
  });
}

export function useCreateArtifact() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateArtifactReq) =>
      api.post<Artifact>(tenantPath("/artifacts"), data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["artifacts"] }),
  });
}

// ---- Compliance ----

export function useFindings(params?: ListParams) {
  return useQuery({
    queryKey: ["findings", params ?? {}],
    queryFn: () =>
      api.get<ListResponse<ComplianceFinding>>(
        tenantPath(`/compliance/findings${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
  });
}

// ---- Desktop Session ----

export function useLivekitConfig() {
  return useQuery({
    queryKey: ["livekitConfig"],
    queryFn: () => api.get<LivekitConfigResponse>("/v1/config/livekit"),
    staleTime: 60_000,
  });
}

// ---- Admin / Config / Auditing ----

export function useSystemSetting(key: string | undefined) {
  return useQuery({
    queryKey: ["systemSetting", key],
    queryFn: () =>
      api.get<SystemSetting>(`/v1/config/settings/${encodeURIComponent(String(key))}`),
    enabled: !!key,
    retry: false,
  });
}

export function useUpsertSystemSetting(key: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: SystemSettingUpsertReq) =>
      api.put<SystemSetting>(`/v1/config/settings/${encodeURIComponent(key)}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["systemSetting", key] });
    },
  });
}

export function useAuditLogs(params?: AuditLogListParams) {
  const q: Record<string, string | number | undefined> = {
    limit: params?.limit,
    offset: params?.offset,
    action: params?.action,
    resource_type: params?.resource_type,
  };
  return useQuery({
    queryKey: ["auditLogs", params ?? {}],
    queryFn: () =>
      api.get<ListResponse<AuditLog>>(
        tenantPath(`/audit-logs${buildQuery(q)}`),
      ),
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useRbacRoles() {
  return useQuery({
    queryKey: ["rbacRoles"],
    queryFn: () => api.get<RbacRole[]>("/v1/config/rbac/roles"),
    retry: false,
  });
}

export function usePolicyEditorPublish() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: PolicyEditorPublishReq) =>
      api.post<PolicyRevision>(tenantPath(`/policies/editor`), body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["policies"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

export function useCreateDesktopSession() {
  return useMutation({
    mutationFn: ({
      deviceId,
      body,
    }: {
      deviceId: string;
      body?: DesktopSessionCreateReq;
    }) =>
      api.post<DesktopSessionResponse>(
        tenantPath(`/devices/${deviceId}/desktop/session`),
        body ?? {},
      ),
  });
}

export function useDeleteDesktopSession() {
  return useMutation({
    mutationFn: ({
      deviceId,
      sessionId,
    }: {
      deviceId: string;
      sessionId: string;
    }) =>
      api.del(
        tenantPath(
          `/devices/${deviceId}/desktop/session${buildQuery({ session_id: sessionId })}`,
        ),
      ),
  });
}

// ---- CSV export helper ----

export function exportCsv<T extends Record<string, unknown>>(
  items: T[],
  filename: string,
) {
  if (!items.length) return;
  const keys = Object.keys(items[0]);
  const header = keys.join(",");
  const rows = items.map((item) =>
    keys
      .map((k) => {
        const v = item[k];
        const s = v === null || v === undefined ? "" : String(v);
        return s.includes(",") || s.includes('"')
          ? `"${s.replace(/"/g, '""')}"`
          : s;
      })
      .join(","),
  );
  const blob = new Blob([header + "\n" + rows.join("\n")], {
    type: "text/csv",
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
