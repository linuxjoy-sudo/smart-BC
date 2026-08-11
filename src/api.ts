import { invoke } from "@tauri-apps/api/core";

export interface RecordResult { conversation_id: number; transcript: string; }
export interface ConversationRow { id: number; created_at: string; transcript: string; audio_path: string | null; }
export interface ReminderRow { id: number; content: string; due_at: string | null; status: string; needs_time: boolean; conversation_id: number; }
export interface PersonRow { id: number; name: string; relation: string; note: string; conversation_id: number; }
export interface PreferenceRow { id: number; topic: string; value: string; conversation_id: number; }

export const api = {
  ping: () => invoke<string>("ping"),
  startRecording: () => invoke<void>("start_recording"),
  stopRecording: () => invoke<RecordResult>("stop_recording"),
  transcriptionReady: () => invoke<boolean>("get_transcription_status"),
  queryMemories: (q: string) => invoke<string>("query_memories", { question: q }),
  listConversations: () => invoke<ConversationRow[]>("list_conversations"),
  listReminders: () => invoke<ReminderRow[]>("list_reminders_cmd"),
  listPeople: () => invoke<PersonRow[]>("list_people_cmd"),
  listPreferences: () => invoke<PreferenceRow[]>("list_preferences_cmd"),
  completeReminder: (id: number) => invoke<void>("complete_reminder", { id }),
};
