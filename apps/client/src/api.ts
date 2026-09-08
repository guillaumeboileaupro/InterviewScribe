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
  current_text: string;
  confidence: number | null;
  status: "raw" | "uncertain";
}

export interface InterviewDetail {
  interview: Interview;
  speakers: Speaker[];
  segments: Segment[];
}

export interface RecoveryInspection {
  interview_id: number;
  audio_duration_ms: number;
  last_stable_end_ms: number;
  remaining_ms: number;
  segment_count: number;
}

export interface Edit {
  id: number;
  segment_id: number;
  operation: "cleanup" | "manual";
  before_text: string;
  after_text: string;
  created_at: string;
  reverted_at: string | null;
}

export interface DiffPart {
  kept: boolean;
  text: string;
  reason: "hesitation" | "repetition" | "pause" | null;
}

export interface CleanupOutcome {
  cleaned_text: string;
  parts: DiffPart[];
}

export interface CleanupApplied {
  segment: Segment;
  outcome: CleanupOutcome;
}

export type ModelStatus = {
  state: "Ready";
  path: string;
  name: string;
  size_mb: number;
};

export interface ModelOption {
  id: string;
  name: string;
  size_mb: number;
}

export function listAvailableModels(): Promise<ModelOption[]> {
  return invoke("list_available_models");
}

export type ExportFormat =
  "txt" | "markdown" | "json" | "srt" | "vtt" | "docx" | "pdf";

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

export function listRecoveryCandidates(): Promise<Interview[]> {
  return invoke("list_recovery_candidates");
}

export function inspectRecoveryCandidate(
  interviewId: number,
): Promise<RecoveryInspection> {
  return invoke("inspect_recovery_candidate", { interviewId });
}

export function recoverInterview(
  interviewId: number,
  modelId?: string,
): Promise<InterviewDetail> {
  return invoke("recover_interview", { interviewId, modelId });
}

export function keepInterruptedAsIs(
  interviewId: number,
): Promise<InterviewDetail> {
  return invoke("keep_interrupted_as_is", { interviewId });
}

export function getInterview(interviewId: number): Promise<InterviewDetail> {
  return invoke("get_interview", { interviewId });
}

export function deleteInterview(interviewId: number): Promise<void> {
  return invoke("delete_interview", { interviewId });
}

export function ensureWhisperModel(): Promise<ModelStatus> {
  return invoke("ensure_whisper_model");
}

export function transcribeInterview(
  interviewId: number,
  expectedSpeakerCount?: number,
  modelId?: string,
): Promise<InterviewDetail> {
  return invoke("transcribe_interview", {
    interviewId,
    expectedSpeakerCount,
    modelId,
  });
}

export function renameSpeaker(
  speakerId: number,
  displayName: string,
): Promise<Speaker> {
  return invoke("rename_speaker", { speakerId, displayName });
}

export function mergeSpeakers(
  interviewId: number,
  keepId: number,
  removeId: number,
): Promise<Speaker[]> {
  return invoke("merge_speakers", { interviewId, keepId, removeId });
}

export function createSpeaker(
  interviewId: number,
  label: string,
): Promise<Speaker> {
  return invoke("create_speaker", { interviewId, label });
}

export function reassignSegmentSpeaker(
  segmentId: number,
  speakerId: number | null,
): Promise<Segment> {
  return invoke("reassign_segment_speaker", { segmentId, speakerId });
}

export function applySegmentCleanup(
  segmentId: number,
): Promise<CleanupApplied> {
  return invoke("apply_segment_cleanup", { segmentId });
}

export function saveSegmentEdit(
  segmentId: number,
  text: string,
): Promise<Segment> {
  return invoke("save_segment_edit", { segmentId, text });
}

export function undoSegmentEdit(segmentId: number): Promise<Segment> {
  return invoke("undo_segment_edit", { segmentId });
}

export function listSegmentEdits(segmentId: number): Promise<Edit[]> {
  return invoke("list_segment_edits", { segmentId });
}

export function exportInterview(
  interviewId: number,
  format: ExportFormat,
  showTimestamps: boolean,
  useCleanedText: boolean,
  destinationPath: string,
): Promise<void> {
  return invoke("export_interview", {
    interviewId,
    format,
    showTimestamps,
    useCleanedText,
    destinationPath,
  });
}

export function checkDocExportAvailable(): Promise<boolean> {
  return invoke("check_doc_export_available");
}

export function exportInterviewDoc(
  interviewId: number,
  showTimestamps: boolean,
  useCleanedText: boolean,
  destinationPath: string,
): Promise<void> {
  return invoke("export_interview_doc", {
    interviewId,
    showTimestamps,
    useCleanedText,
    destinationPath,
  });
}

export function listInputDevices(): Promise<string[]> {
  return invoke("list_input_devices");
}

export function startRecording(
  title: string,
  deviceName?: string,
  expectedSpeakerCount?: number,
  modelId?: string,
): Promise<Interview> {
  return invoke("start_recording", {
    title,
    deviceName,
    expectedSpeakerCount,
    modelId,
  });
}

export function pauseRecording(): Promise<void> {
  return invoke("pause_recording");
}

export function resumeRecording(deviceName?: string): Promise<void> {
  return invoke("resume_recording", { deviceName });
}

export function stopRecording(): Promise<InterviewDetail> {
  return invoke("stop_recording");
}

export function readRecentLogs(): Promise<string> {
  return invoke("read_recent_logs");
}

export function clientLog(level: string, message: string): Promise<void> {
  return invoke("client_log", { level, message });
}
