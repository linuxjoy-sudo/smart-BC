import { invoke } from "@tauri-apps/api/core";

export interface RecordResult { conversation_id: number; transcript: string; }
export interface ConversationRow { id: number; created_at: string; transcript: string; audio_path: string | null; }
export interface ReminderRow { id: number; content: string; due_at: string | null; status: string; needs_time: boolean; conversation_id: number; }
export interface PersonRow { id: number; name: string; relation: string; note: string; conversation_id: number; }
export interface PreferenceRow { id: number; topic: string; value: string; conversation_id: number; }
export interface Config { api_key: string; input_device: number | null; voice_assistant_enabled?: boolean; }
export interface AudioDevice { index: number; name: string; }
export interface VoiceStatus { enabled: boolean; state: string; }

export const api = {
  ping: () => invoke<string>("ping"),
  startRecording: (deviceIndex?: number) =>
    invoke<void>("start_recording", deviceIndex !== undefined ? { deviceIndex } : {}),
  stopRecording: () => invoke<RecordResult>("stop_recording"),
  transcriptionReady: () => invoke<boolean>("get_transcription_status"),
  listAudioDevices: () => invoke<AudioDevice[]>("list_audio_devices"),
  saveInputDevice: (index: number | null) => invoke<void>("save_input_device", { index }),
  queryMemories: (q: string) => invoke<string>("query_memories", { question: q }),
  listConversations: () => invoke<ConversationRow[]>("list_conversations"),
  listReminders: () => invoke<ReminderRow[]>("list_reminders_cmd"),
  listPeople: () => invoke<PersonRow[]>("list_people_cmd"),
  listPreferences: () => invoke<PreferenceRow[]>("list_preferences_cmd"),
  completeReminder: (id: number) => invoke<void>("complete_reminder", { id }),
  saveApiKey: (key: string) => invoke<void>("save_api_key", { key }),
  getConfig: () => invoke<Config>("get_config"),
  clearAllData: () => invoke<void>("clear_all_data"),
  exportAll: (dest: string) => invoke<string>("export_all", { dest }),
  exportDir: () => invoke<string>("export_dir"),
  loadModel: () => invoke<void>("load_model"),
  downloadModel: () => invoke<string>("download_model"),
  logUsage: (event: string) => invoke<void>("log_usage", { event }),
  getUsageStats: () => invoke<{ recordings: number; queries: number; reminder_clicks: number; last_7d_active_days: number }>("get_usage_stats"),
  setVoiceAssistant: (enabled: boolean) => invoke<string>("set_voice_assistant", { enabled }),
  getVoiceStatus: () => invoke<VoiceStatus>("get_voice_status"),
};
