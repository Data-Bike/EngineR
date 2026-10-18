/**
 * useStagedUpload — composable for the staged file upload protocol (docs/15 §3).
 *
 * Splits a file into chunks, initializes the upload, uploads chunks in parallel,
 * completes the upload, and returns the resulting object_id.
 */

import { ref } from 'vue';
import { useApi } from './useApi';

export interface StagedUploadOptions {
  chunkSize?: number;
  hashAlgorithm?: string;
  storageClass?: string;
  metadata?: Record<string, string>;
}

export interface StagedUploadState {
  uploadId: string;
  totalChunks: number;
  receivedChunks: number;
  progress: number;
  status: string;
  objectId: string | null;
}

const DEFAULT_CHUNK_SIZE = 1024 * 1024; // 1 MB

async function computeHash(file: File, algorithm: string): Promise<string> {
  const webCryptoAlgorithm = algorithm.toLowerCase().replace('-', '') === 'sha256'
    ? 'SHA-256'
    : algorithm;
  const digest = await crypto.subtle.digest(webCryptoAlgorithm, await file.arrayBuffer());
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
}

export function useStagedUpload() {
  const api = useApi();
  const state = ref<StagedUploadState | null>(null);
  const uploading = ref(false);
  const error = ref<string | null>(null);

  async function uploadFile(file: File, options: StagedUploadOptions = {}): Promise<string | null> {
    const chunkSize = options.chunkSize ?? DEFAULT_CHUNK_SIZE;
    const hashAlgorithm = options.hashAlgorithm ?? 'sha256';
    uploading.value = true;
    error.value = null;
    state.value = null;

    try {
      const expectedHash = await computeHash(file, hashAlgorithm);
      const totalChunks = Math.max(1, Math.ceil(file.size / chunkSize));

      const initRes = await api.files.uploadInit({
        filename: file.name,
        content_type: file.type || 'application/octet-stream',
        expected_size: file.size,
        expected_hash: expectedHash,
        hash_algorithm: hashAlgorithm,
        chunk_size: chunkSize,
        metadata: options.metadata,
        storage_class: options.storageClass,
      });

      const uploadId = initRes.data.upload_id;
      state.value = {
        uploadId,
        totalChunks,
        receivedChunks: 0,
        progress: 0,
        status: 'uploading',
        objectId: null,
      };

      // Upload chunks sequentially to keep order simple; parallel is possible.
      for (let i = 0; i < totalChunks; i++) {
        const start = i * chunkSize;
        const end = Math.min(file.size, start + chunkSize);
        const chunk = file.slice(start, end);
        const res = await api.files.uploadChunk(uploadId, i, await chunk.arrayBuffer());
        state.value.receivedChunks = res.data.received_chunks as number;
        state.value.progress = (state.value.receivedChunks / totalChunks) * 100;
      }

      const completeRes = await api.files.uploadComplete(uploadId, {
        final_hash: expectedHash,
        total_size: file.size,
      });

      const objectId = completeRes.data.object_id;
      state.value.objectId = objectId;
      state.value.status = 'completed';
      state.value.progress = 100;
      return objectId;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      return null;
    } finally {
      uploading.value = false;
    }
  }

  async function abort(uploadId: string, reason = 'user_cancelled') {
    try {
      await api.files.uploadAbort(uploadId, { reason });
      if (state.value && state.value.uploadId === uploadId) {
        state.value.status = 'aborted';
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  return {
    state,
    uploading,
    error,
    uploadFile,
    abort,
  };
}
