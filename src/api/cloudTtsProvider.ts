import { isTauri } from './env';
import type {
  CloudTtsProviderConfig,
  CloudTtsSynthesizeRequest,
  CreateCloudTtsProviderRequest,
  UpdateCloudTtsProviderRequest,
  VerifyCloudTtsResponse,
  WusoundQuota,
} from '@/types';

function tauriInvoke() {
  if (!isTauri()) throw new Error('Not in Tauri environment');
  return import('@tauri-apps/api/core').then(m => m.invoke);
}

export async function listCloudTtsProviders(): Promise<CloudTtsProviderConfig[]> {
  const invoke = await tauriInvoke();
  return invoke('list_cloud_tts_providers');
}

export async function getCloudTtsProvider(id: string): Promise<CloudTtsProviderConfig> {
  const invoke = await tauriInvoke();
  return invoke('get_cloud_tts_provider', { id });
}

export async function createCloudTtsProvider(request: CreateCloudTtsProviderRequest): Promise<CloudTtsProviderConfig> {
  const invoke = await tauriInvoke();
  return invoke('create_cloud_tts_provider', { request });
}

export async function updateCloudTtsProvider(id: string, request: UpdateCloudTtsProviderRequest): Promise<CloudTtsProviderConfig> {
  const invoke = await tauriInvoke();
  return invoke('update_cloud_tts_provider', { id, request });
}

export async function deleteCloudTtsProvider(id: string): Promise<void> {
  const invoke = await tauriInvoke();
  return invoke('delete_cloud_tts_provider', { id });
}

export async function verifyCloudTtsProvider(id: string): Promise<VerifyCloudTtsResponse> {
  const invoke = await tauriInvoke();
  return invoke('verify_cloud_tts_provider', { id });
}

export async function checkCloudTtsQuota(id: string): Promise<WusoundQuota | null> {
  const invoke = await tauriInvoke();
  return invoke('check_cloud_tts_quota', { id });
}

export async function cloudTtsSynthesize(request: CloudTtsSynthesizeRequest): Promise<Uint8Array> {
  const invoke = await tauriInvoke();
  const bytes = await invoke<number[]>('cloud_tts_synthesize', { request });
  return new Uint8Array(bytes);
}

export async function cloudTtsPreview(request: CloudTtsSynthesizeRequest): Promise<Uint8Array> {
  const invoke = await tauriInvoke();
  const bytes = await invoke<number[]>('cloud_tts_preview', { request });
  return new Uint8Array(bytes);
}
