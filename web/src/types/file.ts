/**
 * Files API types (docs/15 §1-3).
 */

/** Upload init request body (docs/15 §3 step 1). */
export interface UploadInitBody {
  filename: string;
  content_type: string;
  expected_size: number;
  expected_hash: string;
  hash_algorithm: string;
  chunk_size: number;
  metadata?: Record<string, string>;
  storage_class?: string;
}

/** Upload init response. */
export interface UploadInitResponse {
  upload_id: string;
  total_chunks: number;
  status: string;
}

/** Upload complete request body. */
export interface UploadCompleteBody {
  final_hash: string;
  total_size: number;
}

/** Upload abort request body. */
export interface UploadAbortBody {
  reason: string;
}

/** Upload status response. */
export interface UploadStatusResponse {
  upload_id: string;
  status: string;
  progress: number;
  received_chunks: number;
  total_chunks: number;
  object_id: string | null;
}

/** File metadata response. */
export interface FileMetadataResponse {
  object_id: string;
  filename: string;
  content_type: string;
  size: number;
  content_hash: string;
  created_at: string;
  classification: string;
}

/** Document qualifier attached to an entity document. */
export interface DocumentQualifier {
  label: string;
  value: string;
}

/** Entity document returned by GET /entities/{entityId}/documents. */
export interface ApiDocument {
  id: string;
  title: string;
  kind: string;
  file_name: string;
  size_bytes: number;
  content_type: string;
  uploaded_at: string;
  qualifiers: DocumentQualifier[];
  valid_from?: string;
  valid_to?: string;
  source?: string;
}

/** Upload status (docs/15 §3). */
export const UploadStatus = {
  Initiated: 'initiated',
  Uploading: 'uploading',
  AllChunksReceived: 'all_chunks_received',
  Completed: 'completed',
  Aborted: 'aborted',
  ScanFailed: 'scan_failed',
} as const;

export type UploadStatus = (typeof UploadStatus)[keyof typeof UploadStatus];

/** Storage class (docs/15 §1). */
export const StorageClass = {
  Hot: 'hot',
  Cool: 'cool',
  Archive: 'archive',
} as const;

export type StorageClass = (typeof StorageClass)[keyof typeof StorageClass];
