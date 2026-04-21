// ── Auth ─────────────────────────────────────────────────────────────────

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  email: string;
  password: string;
}

export interface TokenPair {
  access_token: string;
  refresh_token: string;
  expires_in: number;
}

export interface User {
  id: string;
  username: string;
  email: string;
  is_active: boolean;
  role_ids: string[];
  roles: string[];
  created_at: string;
  updated_at: string;
}

// ── Content Types ────────────────────────────────────────────────────────

export type FieldTypeKind =
  | 'text'
  | 'richtext'
  | 'number'
  | 'boolean'
  | 'datetime'
  | 'json'
  | 'media'
  | 'relation'
  | 'custom';

export type RelationKind = 'one_to_one' | 'one_to_many' | 'many_to_many';

export type FieldType =
  | { kind: 'text' }
  | { kind: 'richtext' }
  | { kind: 'number'; decimal?: boolean }
  | { kind: 'boolean' }
  | { kind: 'datetime' }
  | { kind: 'json' }
  | { kind: 'media'; allowed_types?: string[] }
  | {
      kind: 'relation';
      target_type: string;
      relation_kind: RelationKind;
    }
  | { kind: 'custom'; type_name: string };

export interface ValidationRule {
  rule: string;
  value?: unknown;
  pattern?: string;
  values?: unknown[];
  validator?: string;
}

export interface FieldDefinition {
  name: string;
  api_id: string;
  field_type: FieldType;
  required: boolean;
  unique: boolean;
  default_value?: unknown;
  description?: string;
  validations: ValidationRule[];
  position: number;
}

export interface ContentTypeDefinition {
  id: string;
  name: string;
  api_id: string;
  description?: string;
  kind: 'collection' | 'single';
  fields: FieldDefinition[];
  created_at: string;
  updated_at: string;
}

export interface CreateContentTypeInput {
  name: string;
  api_id: string;
  description?: string;
  kind?: 'collection' | 'single';
  fields: FieldDefinition[];
}

export interface UpdateContentTypeInput {
  name?: string;
  description?: string | null;
  fields?: FieldDefinition[];
}

// ── Content Entries ──────────────────────────────────────────────────────

export type ContentStatus = 'draft' | 'published' | 'archived';

export interface ContentEntry {
  id: string;
  content_type_id: string;
  content_type_api_id: string;
  slug?: string;
  status: ContentStatus;
  current_version_id?: string;
  published_version_id?: string;
  fields: Record<string, unknown>;
  created_by: string;
  updated_by: string;
  created_at: string;
  updated_at: string;
  published_at?: string;
  populated?: Record<string, ContentEntry[]>;
}

export interface CreateEntryInput {
  data: Record<string, unknown>;
  slug?: string;
}

export interface UpdateEntryInput {
  data?: Record<string, unknown>;
  slug?: string | null;
}

export interface PaginationMeta {
  page: number;
  page_size: number;
  page_count: number;
  total: number;
}

export interface PaginatedResponse<T> {
  data: T[];
  meta: PaginationMeta;
}

// ── Media ────────────────────────────────────────────────────────────────

export interface MediaAsset {
  id: string;
  filename: string;
  original_filename: string;
  mime_type: string;
  size: number;
  storage_path: string;
  metadata?: Record<string, unknown>;
  uploaded_by: string;
  created_at: string;
}

export interface MediaListResponse {
  data: MediaAsset[];
  total: number;
  page: number;
  page_size: number;
  page_count: number;
}

// ── Plugins ──────────────────────────────────────────────────────────────

export type PluginStatus = 'discovered' | 'installed' | 'enabled' | 'disabled' | 'error';

export interface Plugin {
  name: string;
  version: string;
  description?: string;
  status: PluginStatus;
  runtime: string;
  installed_at?: string;
  enabled_at?: string;
}

// ── Roles & Permissions ──────────────────────────────────────────────────

export interface Permission {
  id: string;
  domain: string;
  resource: string;
  action: string;
  scope: 'all' | 'own';
  source: string;
}

export interface Role {
  id: string;
  name: string;
  description?: string;
  is_system: boolean;
  created_at: string;
  permissions: Permission[];
}

export interface CreateRoleInput {
  name: string;
  description?: string;
  permission_ids: string[];
}

export interface UpdateRoleInput {
  name?: string;
  description?: string | null;
  permission_ids?: string[];
}

export interface CreateUserInput {
  username: string;
  email: string;
  password: string;
  is_active?: boolean;
  role_ids?: string[];
}

export interface UpdateUserInput {
  username?: string;
  email?: string;
  password?: string;
  is_active?: boolean;
  role_ids?: string[];
}

// ── Settings ─────────────────────────────────────────────────────────────

export interface SettingsEntry {
  id: string;
  namespace: string;
  key: string;
  value: unknown;
  updated_at: string;
}

export interface JsonSchemaNode {
  type?: string | string[];
  title?: string;
  description?: string;
  default?: unknown;
  enum?: unknown[];
  format?: string;
  properties?: Record<string, JsonSchemaNode>;
  required?: string[];
  items?: JsonSchemaNode;
  [key: string]: unknown;
}

export interface PluginSchema {
  plugin_name: string;
  schema: JsonSchemaNode;
  created_at: string;
}

export interface ContributionMatchSpec {
  contentTypeApiIds: string[];
}

export interface ExtensionDiagnostic {
  pluginName: string;
  pluginVersion: string;
  severity: string;
  code: string;
  message: string;
}

export interface BootstrapMenuContribution {
  id: string;
  label: string;
  zone: string;
  icon?: string;
  order: number;
  to: string;
  fullPath: string;
  requiredPermissions: string[];
}

export interface BootstrapRouteContribution {
  id: string;
  path: string;
  fullPath: string;
  moduleUrl: string;
  styles: string[];
  kind: string;
  title: string;
  requiredPermissions: string[];
  match: ContributionMatchSpec;
}

export interface BootstrapSlotContribution {
  id: string;
  slot: string;
  order: number;
  moduleUrl: string;
  styles: string[];
  requiredPermissions: string[];
  match: ContributionMatchSpec;
}

export interface BootstrapFieldRendererContribution {
  id: string;
  typeName: string;
  moduleUrl: string;
  styles: string[];
  requiredPermissions: string[];
}

export interface BootstrapSettingsPage {
  path: string;
  fullPath: string;
  moduleUrl: string;
  styles: string[];
}

export interface BootstrapSettingsContribution {
  namespace: string;
  requiredPermissions: string[];
  customPage: BootstrapSettingsPage | null;
}

export interface BootstrapPlugin {
  name: string;
  version: string;
  menus: BootstrapMenuContribution[];
  routes: BootstrapRouteContribution[];
  slots: BootstrapSlotContribution[];
  fieldRenderers: BootstrapFieldRendererContribution[];
  settings: BootstrapSettingsContribution | null;
}

export interface AdminExtensionBootstrap {
  revision: string;
  shellSdkVersion: string;
  plugins: BootstrapPlugin[];
  diagnostics: ExtensionDiagnostic[];
}

export interface AdminExtensionDiagnostics {
  revision: string;
  diagnostics: ExtensionDiagnostic[];
}

// ── Revisions ────────────────────────────────────────────────────────────

export interface Revision {
  id: string;
  content_entry_id: string;
  version: number;
  data: Record<string, unknown>;
  slug?: string;
  actor_id: string;
  created_at: string;
}

export interface RevisionListResponse {
  data: Revision[];
  total: number;
  page: number;
  page_size: number;
}

// ── API Error ────────────────────────────────────────────────────────────

export interface ApiErrorBody {
  error: {
    status: number;
    name: string;
    code: string;
    message: string;
    details?: unknown;
  };
}

// ── Public Content Types ─────────────────────────────────────────────────

export interface PublicContentType {
  id: string;
  name: string;
  api_id: string;
  description?: string;
}
