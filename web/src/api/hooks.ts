import {
  useQuery,
  useMutation,
  useQueryClient,
  keepPreviousData,
} from "@tanstack/react-query";
import { useAppSession } from "../appProviders";
import { api, tenantPathFor, buildQuery } from "./client";
import type {
  LoginResponse,
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
  TenantRbacRolesResponse,
  TenantRbacRolesUpsertReq,
  TenantRoleBindingsResponse,
  TenantRoleBindingsUpsertReq,
  TenantRbacMeResponse,
  Tenant,
  PlatformTenantSummary,
  PlatformQuota,
  PlatformHealth,
  DeviceEnrollmentToken,
  IssueDeviceEnrollmentTokenReq,
  BatchCreateDevicesReq,
  BatchCreateDevicesResponse,
  DeviceEnrollmentBatchResponse,
  DeviceEnrollmentBatchSummary,
} from "./types";

// ---- Dashboard ----

export function useStats() {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["stats", tenantId],
    queryFn: () => api.get<DashboardStats>(tenantPathFor(tenantId, "/stats")),
    refetchInterval: 15_000,
  });
}

// ---- Devices ----

export function useDevices(params?: ListParams) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["devices", tenantId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Device>>(
        tenantPathFor(tenantId, `/devices${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
    refetchInterval: 10_000,
  });
}

export function useDevice(id: string | undefined) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["device", tenantId, id],
    queryFn: () => api.get<Device>(tenantPathFor(tenantId, `/devices/${id}`)),
    enabled: !!id,
  });
}

export function useCreateDevice() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateDeviceReq) =>
      api.post<Device>(tenantPathFor(tenantId, "/devices"), data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["devices", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

export function useBatchCreateDevices() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: BatchCreateDevicesReq) =>
      api.post<BatchCreateDevicesResponse>(
        tenantPathFor(tenantId, "/devices:batch-create"),
        data,
      ),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["devices", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

export function useDeviceEnrollmentBatch(batchId?: string | null) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["deviceEnrollmentBatch", tenantId, batchId],
    queryFn: () =>
      api.get<DeviceEnrollmentBatchResponse>(
        tenantPathFor(tenantId, `/device-enrollment-batches/${batchId}`),
      ),
    enabled: Boolean(batchId),
    retry: false,
  });
}

export function useDeviceEnrollmentBatches(params?: ListParams) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["deviceEnrollmentBatches", tenantId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<DeviceEnrollmentBatchSummary>>(
        tenantPathFor(tenantId, `/device-enrollment-batches${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useDeleteDevice() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.del(tenantPathFor(tenantId, `/devices/${id}`)),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["devices", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

export function useRotateDeviceRegistrationCode() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (deviceId: string) =>
      api.post<Device>(tenantPathFor(tenantId, `/devices/${deviceId}/registration-code:rotate`), {}),
    onSuccess: (device) => {
      qc.invalidateQueries({ queryKey: ["devices", tenantId] });
      qc.invalidateQueries({ queryKey: ["device", tenantId, device.id] });
    },
  });
}

export function useIssueDeviceEnrollmentToken() {
  const { tenantId } = useAppSession();
  return useMutation({
    mutationFn: ({
      deviceId,
      body,
    }: {
      deviceId: string;
      body: IssueDeviceEnrollmentTokenReq;
    }) =>
      api.post<DeviceEnrollmentToken>(
        tenantPathFor(tenantId, `/devices/${deviceId}/enrollment-token`),
        body,
      ),
  });
}

// ---- Policies ----

export function usePolicies(params?: ListParams) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["policies", tenantId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Policy>>(
        tenantPathFor(tenantId, `/policies${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
  });
}

export function usePolicy(id: string | undefined) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["policy", tenantId, id],
    queryFn: () => api.get<Policy>(tenantPathFor(tenantId, `/policies/${id}`)),
    enabled: !!id,
  });
}

export function useCreatePolicy() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreatePolicyReq) =>
      api.post<Policy>(tenantPathFor(tenantId, "/policies"), data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["policies", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

export function useDeletePolicy() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.del(tenantPathFor(tenantId, `/policies/${id}`)),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["policies", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

// ---- Commands ----

export function useCommands(params?: ListParams) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["commands", tenantId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Command>>(
        tenantPathFor(tenantId, `/commands${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
    refetchInterval: 10_000,
  });
}

export function useCommand(id: string | undefined) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["command", tenantId, id],
    queryFn: () => api.get<Command>(tenantPathFor(tenantId, `/commands/${id}`)),
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
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateCommandReq) =>
      api.post<Command>(tenantPathFor(tenantId, "/commands"), data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["commands", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

// ---- Device Shadow ----

export function useShadow(deviceId: string | undefined) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["shadow", tenantId, deviceId],
    queryFn: () =>
      api.get<ShadowResponse>(tenantPathFor(tenantId, `/devices/${deviceId}/shadow`)),
    enabled: !!deviceId,
    refetchInterval: 10_000,
  });
}

export function useUpdateShadowDesired() {
  const { tenantId } = useAppSession();
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
          tenantPathFor(tenantId, `/devices/${deviceId}/shadow/desired`),
          desired,
        ),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["shadow", tenantId, vars.deviceId] });
    },
  });
}

// ---- Device Actions (remote control) ----

export function useDeviceAction() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      deviceId,
      action,
    }: {
      deviceId: string;
      action: DeviceActionReq;
    }) =>
      api.post<Command>(tenantPathFor(tenantId, `/devices/${deviceId}/actions`), action),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["deviceCommands", tenantId, vars.deviceId] });
      qc.invalidateQueries({ queryKey: ["commands", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

export function useDeviceCommands(
  deviceId: string | undefined,
  params?: ListParams,
) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["deviceCommands", tenantId, deviceId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Command>>(
        tenantPathFor(tenantId, `/devices/${deviceId}/commands${buildQuery(params)}`),
      ),
    enabled: !!deviceId,
    placeholderData: keepPreviousData,
    refetchInterval: 10_000,
  });
}

// ---- Command Result ----

export function useCommandResult(commandId: string | undefined) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["commandResult", tenantId, commandId],
    queryFn: () =>
      api.get<CommandResult>(tenantPathFor(tenantId, `/commands/${commandId}/result`)),
    enabled: !!commandId,
    retry: false,
    refetchInterval: (query) =>
      commandId && !query.state.data ? 3_000 : false,
  });
}

export function useSubmitCommandResult() {
  const { tenantId } = useAppSession();
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
        tenantPathFor(tenantId, `/commands/${commandId}/result`),
        body,
      ),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["commandResult", tenantId, vars.commandId] });
      qc.invalidateQueries({ queryKey: ["command", tenantId, vars.commandId] });
      qc.invalidateQueries({ queryKey: ["commands", tenantId] });
    },
  });
}

export function useUpdateCommandStatus() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      commandId,
      body,
    }: {
      commandId: string;
      body: UpdateCommandStatusReq;
    }) =>
      api.patch<Command>(tenantPathFor(tenantId, `/commands/${commandId}/status`), body),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ["command", tenantId, vars.commandId] });
      qc.invalidateQueries({ queryKey: ["commands", tenantId] });
    },
  });
}

// ---- Artifacts ----

export function useArtifacts(params?: ListParams, options?: { tenantId?: string; enabled?: boolean }) {
  const session = useAppSession();
  const tenantId = options?.tenantId ?? session.tenantId;
  return useQuery({
    queryKey: ["artifacts", tenantId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<Artifact>>(
        tenantPathFor(tenantId, `/artifacts${buildQuery(params)}`),
      ),
    placeholderData: keepPreviousData,
    enabled: options?.enabled ?? true,
  });
}

export function useCreateArtifact() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateArtifactReq) =>
      api.post<Artifact>(tenantPathFor(tenantId, "/artifacts"), data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["artifacts", tenantId] }),
  });
}

// ---- Compliance ----

export function useFindings(params?: ListParams) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["findings", tenantId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<ComplianceFinding>>(
        tenantPathFor(tenantId, `/compliance/findings${buildQuery(params)}`),
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
  const { tenantId } = useAppSession();
  const q: Record<string, string | number | undefined> = {
    limit: params?.limit,
    offset: params?.offset,
    action: params?.action,
    resource_type: params?.resource_type,
  };
  return useQuery({
    queryKey: ["auditLogs", tenantId, params ?? {}],
    queryFn: () =>
      api.get<ListResponse<AuditLog>>(
        tenantPathFor(tenantId, `/audit-logs${buildQuery(q)}`),
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

export function useTenantRbacRoles() {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["tenantRbacRoles", tenantId],
    queryFn: () =>
      api.get<TenantRbacRolesResponse>(tenantPathFor(tenantId, "/rbac/roles")),
    retry: false,
  });
}

export function useUpsertTenantRbacRoles() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: TenantRbacRolesUpsertReq) =>
      api.put<TenantRbacRolesResponse>(tenantPathFor(tenantId, "/rbac/roles"), body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["tenantRbacRoles", tenantId] });
      qc.invalidateQueries({ queryKey: ["rbacRoles"] });
    },
  });
}

export function useTenantRoleBindings() {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["tenantRoleBindings", tenantId],
    queryFn: () =>
      api.get<TenantRoleBindingsResponse>(tenantPathFor(tenantId, "/rbac/bindings")),
    retry: false,
  });
}

export function useUpsertTenantRoleBindings() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: TenantRoleBindingsUpsertReq) =>
      api.put<TenantRoleBindingsResponse>(tenantPathFor(tenantId, "/rbac/bindings"), body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["tenantRoleBindings", tenantId] });
      qc.invalidateQueries({ queryKey: ["tenantRbacMe", tenantId] });
    },
  });
}

export function useTenantRbacMe(options?: { enabled?: boolean }) {
  const { tenantId } = useAppSession();
  return useQuery({
    queryKey: ["tenantRbacMe", tenantId],
    queryFn: () => api.get<TenantRbacMeResponse>(tenantPathFor(tenantId, "/rbac/me")),
    retry: false,
    enabled: options?.enabled ?? true,
  });
}

export function useCreateTenant() {
  return useMutation({
    mutationFn: (body: { name: string }) => api.post<Tenant>("/v1/tenants", body),
  });
}

export function useLogin() {
  return useMutation({
    mutationFn: (body: { username: string; password: string }) =>
      api.post<LoginResponse>("/v1/auth/login", body),
  });
}

export function useSelectLoginScope() {
  return useMutation({
    mutationFn: (body: {
      username: string;
      login_transaction_token: string;
      scope: "platform" | "tenant";
      tenant_id?: string;
    }) => api.post<LoginResponse>("/v1/auth/login/select", body),
  });
}

export function useLogout() {
  return useMutation({
    mutationFn: (body: { tenant_id?: string }) => api.post<void>("/v1/auth/logout", body),
  });
}

export function usePlatformTenants() {
  return usePlatformTenantsList();
}

export function usePlatformTenantsList(params?: ListParams) {
  return useQuery({
    queryKey: ["platformTenants", params ?? {}],
    queryFn: () =>
      api.get<ListResponse<PlatformTenantSummary>>(
        `/v1/config/tenants${buildQuery(params)}`,
      ),
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function usePlatformAuditLogs(params?: AuditLogListParams) {
  const q: Record<string, string | number | undefined> = {
    limit: params?.limit,
    offset: params?.offset,
    action: params?.action,
    resource_type: params?.resource_type,
  };
  return useQuery({
    queryKey: ["platformAuditLogs", params ?? {}],
    queryFn: () => api.get<ListResponse<AuditLog>>(`/v1/config/audit-logs${buildQuery(q)}`),
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function usePlatformHealth() {
  return useQuery({
    queryKey: ["platformHealth"],
    queryFn: () => api.get<PlatformHealth>("/v1/config/platform-health"),
    refetchInterval: 15_000,
    retry: false,
  });
}

export function usePlatformQuotas() {
  return useQuery({
    queryKey: ["platformQuotas"],
    queryFn: () => api.get<ListResponse<PlatformQuota>>("/v1/config/quotas"),
    retry: false,
  });
}

export function usePolicyEditorPublish() {
  const { tenantId } = useAppSession();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: PolicyEditorPublishReq) =>
      api.post<PolicyRevision>(tenantPathFor(tenantId, `/policies/editor`), body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["policies", tenantId] });
      qc.invalidateQueries({ queryKey: ["stats", tenantId] });
    },
  });
}

export function useCreateDesktopSession() {
  const { tenantId } = useAppSession();
  return useMutation({
    mutationFn: ({
      deviceId,
      body,
    }: {
      deviceId: string;
      body?: DesktopSessionCreateReq;
    }) =>
      api.post<DesktopSessionResponse>(
        tenantPathFor(tenantId, `/devices/${deviceId}/desktop/session`),
        body ?? {},
      ),
  });
}

export function useDeleteDesktopSession() {
  const { tenantId } = useAppSession();
  return useMutation({
    mutationFn: ({
      deviceId,
      sessionId,
    }: {
      deviceId: string;
      sessionId: string;
    }) =>
      api.del(
        tenantPathFor(
          tenantId,
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
