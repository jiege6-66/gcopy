import { invoke } from '@tauri-apps/api/core';

export interface ClipboardContent {
  type: 'Text' | 'Image';
  data: string | number[];
}

export async function readClipboard(): Promise<ClipboardContent> {
  return invoke('read_clipboard');
}

export async function writeClipboard(content: ClipboardContent): Promise<void> {
  return invoke('write_clipboard', { content });
}
