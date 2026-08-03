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

  /** DELETE /files/{id} — delete a file. */
  async deleteFile(id: string): Promise<ParsedResponse<void>> {
    return this.client.delete<void>(`/files/${id}`);
  }
}
