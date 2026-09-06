import { invoke } from "@tauri-apps/api/core";

export interface Interview {
  id: number;
  title: string;
  language: string | null;
  mode: string;
  audio_path: string;
  status: "imported" | "transcribing" | "transcribed" | "error";
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface Speaker {
  id: number;
  interview_id: number;
  label: string;
  color: string;
  display_name: string | null;
}

export interface Segment {
  id: number;
  interview_id: number;
  speaker_id: number | null;
  start_ms: number;
  end_ms: number;
  raw_text: string;
  confidence: number | null;
  status: "raw" | "uncertain";
}

export interface InterviewDetail {
  interview: Interview;
  speakers: Speaker[];
  segments: Segment[];
}

export type ModelStatus = {
  state: "Ready";
  path: string;
  name: string;
  size_mb: number;
};

export type ExportFormat = "txt" | "markdown" | "json";

export function importInterview(
  title: string,
  sourcePath: string,
  language?: string,
): Promise<Interview> {
  return invoke("import_interview", { title, sourcePath, language });
}

export function listInterviews(): Promise<Interview[]> {
  return invoke("list_interviews");
}

export function getInterview(interviewId: number): Promise<InterviewDetail> {
  return invoke("get_interview", { interviewId });
}

export function ensureWhisperModel(): Promise<ModelStatus> {
  return invoke("ensure_whisper_model");
}

export function transcribeInterview(
  interviewId: number,
): Promise<InterviewDetail> {
  return invoke("transcribe_interview", { interviewId });
}

export function exportInterview(
  interviewId: number,
  format: ExportFormat,
  showTimestamps: boolean,
  destinationPath: string,
): Promise<void> {
  return invoke("export_interview", {
    interviewId,
    format,
    showTimestamps,
    destinationPath,
  });
}
