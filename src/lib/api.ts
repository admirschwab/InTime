import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface Project {
  id: number;
  name: string;
  description: string;
  color: string;
  isWork: boolean;
  isDefault: boolean;
  isEnabled: boolean;
  sortIndex: number;
}

export type ProjectInput = Pick<Project, 'name' | 'description' | 'color' | 'isWork' | 'isDefault'>;

/** Times are local wall-clock times: `YYYY-MM-DDTHH:MM:SS`. */
export interface Work {
  id: number;
  projectId: number;
  startTime: string;
  endTime: string;
  notes: string;
  active: boolean;
}

export type WorkInput = Pick<Work, 'projectId' | 'startTime' | 'endTime' | 'notes'>;

export interface Current {
  work: Work;
  /** Finished time per project today, without the running work. */
  today: { projectId: number; seconds: number }[];
}

export interface Settings {
  backgroundColor: string;
  textColor: string;
  hoverBackgroundColor: string;
  hoverTextColor: string;
  displayProjectsRight: boolean;
  hideProjectsOnMouseExit: boolean;
  saveWindowPosition: boolean;
  remindNotes: boolean;
  remindNotesOnlyWork: boolean;
  confirmClose: boolean;
}

export interface HeimatConfig {
  url: string;
  token: string;
}

export interface HeimatTask {
  id: number;
  name: string;
  taskHolderName: string;
  taskHolderType: string;
  isFavorite: boolean;
  bookingHint: string | null;
  isStartAndEndTimeRequired: boolean;
  isNoteOptional: boolean;
}

export interface HeimatMapping {
  projectId: number;
  taskId: number;
  taskName: string;
  taskHolderName: string;
}

export type SyncStatus = 'ready' | 'unmapped' | 'unavailable' | 'heimatOnly';

export interface SyncRow {
  taskId: number | null;
  taskName: string;
  taskHolderName: string;
  bookingHint: string;
  isNoteOptional: boolean;
  status: SyncStatus;
  projectIds: number[];
  intimeSeconds: number;
  intimeNotes: string;
  heimatSeconds: number;
  heimatNotes: string;
  existingTimeIds: number[];
  suggestSync: boolean;
}

export interface SyncItem {
  taskId: number;
  minutes: number;
  note: string;
  existingTimeIds: number[];
}

export const DEFAULT_COLORS = {
  backgroundColor: '#1f2329',
  textColor: '#e8eaed',
  hoverBackgroundColor: '#2d333b',
  hoverTextColor: '#ffffff',
} satisfies Partial<Settings>;

export const api = {
  projects: (includeDisabled = false) => invoke<Project[]>('get_projects', { includeDisabled }),
  createProject: (input: ProjectInput) => invoke<Project>('create_project', { input }),
  updateProject: (id: number, input: ProjectInput) => invoke<Project>('update_project', { id, input }),
  deleteProject: (id: number) => invoke<void>('delete_project', { id }),
  reorderProjects: (ids: number[]) => invoke<void>('reorder_projects', { ids }),

  current: () => invoke<Current>('get_current'),
  switchProject: (projectId: number, notes: string | null, transferMinutes = 0) =>
    invoke<void>('switch_project', { projectId, notes, transferMinutes }),
  updateCurrentNotes: (notes: string) => invoke<void>('update_current_notes', { notes }),

  worksForDay: (date: string) => invoke<Work[]>('get_works_for_day', { date }),
  daysWithWork: (year: number, month: number) => invoke<number[]>('get_days_with_work', { year, month }),
  createWork: (input: WorkInput) => invoke<void>('create_work', { input }),
  updateWork: (id: number, input: WorkInput) => invoke<void>('update_work', { id, input }),
  deleteWork: (id: number) => invoke<void>('delete_work', { id }),

  settings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
  exportData: (path: string) => invoke<void>('export_data', { path }),
  importData: (path: string) => invoke<void>('import_data', { path }),

  heimatConfig: () => invoke<HeimatConfig>('heimat_get_config'),
  saveHeimatConfig: (config: HeimatConfig) => invoke<void>('heimat_save_config', { config }),
  testHeimat: (config: HeimatConfig) => invoke<number>('heimat_test', { config }),
  heimatTasks: () => invoke<HeimatTask[]>('heimat_tasks'),
  heimatMappings: () => invoke<HeimatMapping[]>('heimat_get_mappings'),
  saveHeimatMappings: (mappings: HeimatMapping[]) => invoke<void>('heimat_save_mappings', { mappings }),
  importHeimatTasks: (tasks: HeimatTask[]) => invoke<void>('heimat_import_tasks', { tasks }),
  heimatDay: (date: string) => invoke<SyncRow[]>('heimat_day', { date }),
  heimatSync: (date: string, items: SyncItem[]) =>
    invoke<{ taskId: number; error: string | null }[]>('heimat_sync', { date, items }),

  openWindow: (view: 'report' | 'settings' | 'project' | 'heimat', id: number | null = null) =>
    invoke<void>('open_window', { view, id }),
  quit: () => invoke<void>('quit_app'),
};

export function onDataChanged(callback: () => void): Promise<UnlistenFn> {
  return listen('data-changed', callback);
}

/** Subscribes to an event for the lifetime of a Svelte `$effect`. */
export function subscribe(register: Promise<UnlistenFn>): () => void {
  return () => {
    register.then((unlisten) => unlisten());
  };
}

export function errorText(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
}
