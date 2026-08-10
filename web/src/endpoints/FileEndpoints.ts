/**
 * Files API endpoints (docs/15 §1-3).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type {
  UploadInitBody,
  UploadInitResponse,
  UploadCompleteBody,
  UploadAbortBody,
  UploadStatusResponse,
  FileMetadataResponse,
} from '../types/index.js';

/** Files API endpoint methods. */
export class FileEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** POST /files/upload/init — initialize staged upload. */
  async uploadInit(body: UploadInitBody): Promise<ParsedResponse<UploadInitResponse>> {
    return this.client.post<UploadInitResponse>('/files/upload/init', body);
  }

  /** PUT /files/upload/{id}/chunks/{index} — upload a chunk. */
  async uploadChunk(
    uploadId: string,
    index: number,
    chunk: ArrayBuffer | Blob,
  ): Promise<ParsedResponse<unknown>> {
    return this.client.putRaw<unknown>(`/files/upload/${uploadId}/chunks/${index}`, chunk);
  }

  /** POST /files/upload/{id}/complete — complete upload. */
  async uploadComplete(
    uploadId: string,
    body: UploadCompleteBody,
  ): Promise<ParsedResponse<UploadStatusResponse>> {
    return this.client.post<UploadStatusResponse>(`/files/upload/${uploadId}/complete`, body);
  }

  /** POST /files/upload/{id}/abort — abort upload. */
  async uploadAbort(
    uploadId: string,
    body: UploadAbortBody,
  ): Promise<ParsedResponse<void>> {
    return this.client.post<void>(`/files/upload/${uploadId}/abort`, body);
  }

  /** GET /files/upload/{id} — get upload status. */
  async uploadStatus(uploadId: string): Promise<ParsedResponse<UploadStatusResponse>> {
    return this.client.get<UploadStatusResponse>(`/files/upload/${uploadId}`);
  }

  /** GET /files/{id} — download file metadata. */
  async getFile(id: string): Promise<ParsedResponse<FileMetadataResponse>> {
    return this.client.get<FileMetadataResponse>(`/files/${id}`);
  }

  /** POST /entities/{entityId}/documents — upload a document via simple multipart form. */
  async uploadEntityDocument<T = unknown>(entityId: string, form: FormData): Promise<ParsedResponse<T>> {
    return this.client.postMultipart<T>(`/entities/${entityId}/documents`, form);
  }

  /** GET /entities/{entityId}/documents — list entity documents. */
  async getEntityDocuments(entityId: string): Promise<ParsedResponse<ApiDocument[]>> {
    return this.client.get<ApiDocument[]>(`/entities/${entityId}/documents`);
  }

  /** GET /entities/{entityId}/documents/{docId}/download — download a document as Blob. */
  async downloadEntityDocument(entityId: string, docId: string): Promise<Blob> {
    return this.client.downloadBlob(`/entities/${entityId}/documents/${docId}/download`);
  }

  /** DELETE /files/{id} — delete a file. */
  async deleteFile(id: string): Promise<ParsedResponse<void>> {
    return this.client.delete<void>(`/files/${id}`);
  }
}
