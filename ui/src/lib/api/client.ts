import axios from "axios";

export const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL ?? "",
  timeout: 15_000,
});

export async function getJson<T>(path: string): Promise<T> {
  const response = await apiClient.get<T>(path);
  return response.data;
}

export async function postJson<T>(path: string, body?: unknown): Promise<T> {
  const response = await apiClient.post<T>(path, body);
  return response.data;
}
