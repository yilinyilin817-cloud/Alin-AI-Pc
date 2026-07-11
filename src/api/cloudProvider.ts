import { isTauri } from './env';
import type {
  CloudProviderConfig,
  CreateCloudProviderRequest,
  UpdateCloudProviderRequest,
  VerifyCloudProviderResponse,
} from '@/types';

function tauriInvoke() {
  if (!isTauri()) throw new Error('Not in Tauri environment');
  return import('@tauri-apps/api/core').then(m => m.invoke);
}

export async function listCloudProviders(): Promise<CloudProviderConfig[]> {
  const invoke = await tauriInvoke();
  return invoke('list_cloud_providers');
}

export async function createCloudProvider(
  request: CreateCloudProviderRequest
): Promise<CloudProviderConfig> {
  const invoke = await tauriInvoke();
  return invoke('create_cloud_provider', { request });
}

export async function updateCloudProvider(
  id: string,
  request: UpdateCloudProviderRequest
): Promise<CloudProviderConfig> {
  const invoke = await tauriInvoke();
  return invoke('update_cloud_provider', { id, request });
}

export async function deleteCloudProvider(id: string): Promise<void> {
  const invoke = await tauriInvoke();
  return invoke('delete_cloud_provider', { id });
}

export async function verifyCloudProvider(
  id: string
): Promise<VerifyCloudProviderResponse> {
  const invoke = await tauriInvoke();
  return invoke('verify_cloud_provider', { id });
}

export async function syncCloudModels(providerId: string): Promise<number> {
  const invoke = await tauriInvoke();
  return invoke('sync_cloud_models', { providerId });
}
