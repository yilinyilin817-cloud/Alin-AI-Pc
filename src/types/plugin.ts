export type PluginPermission = 'fs_read' | 'fs_write' | 'network' | 'shell' | 'clipboard';

export interface PluginConfigField {
  key: string;
  label: string;
  type: 'string' | 'number' | 'boolean' | 'select' | 'password';
  default?: unknown;
  options?: { label: string; value: string }[];
  required?: boolean;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  icon?: string;
  entry?: string;
  permissions: PluginPermission[];
  config?: PluginConfigField[];
  skills?: Array<{
    id: string;
    name: string;
    description: string;
    icon?: string;
    arguments?: Record<string, unknown>;
  }>;
  commands?: Array<{
    id: string;
    title: string;
    shortcut?: string;
  }>;
}

export interface InstalledPlugin {
  id: string;
  manifest: PluginManifest;
  enabled: boolean;
  config: Record<string, unknown>;
  installedAt: string;
  updatedAt: string;
  path: string;
}
