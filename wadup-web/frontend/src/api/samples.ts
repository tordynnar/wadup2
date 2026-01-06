import { get, del, upload, post } from './client'
import type { Sample, TestRun } from '../types'

export const samplesApi = {
  list: () =>
    get<Sample[]>('/api/samples'),

  upload: (file: File) =>
    upload<Sample>('/api/samples', file),

  delete: (id: number) =>
    del<void>(`/api/samples/${id}`),
}

export const testApi = {
  run: (moduleId: number, sampleIds: number[]) =>
    post<TestRun[]>(`/api/modules/${moduleId}/test`, { sample_ids: sampleIds }),

  getResult: (moduleId: number, runId: number) =>
    get<TestRun>(`/api/modules/${moduleId}/test/${runId}`),
}
