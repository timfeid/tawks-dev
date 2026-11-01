import type { RoomConnection } from './room.svelte';
import type { Me, ReportReason, SongInfo } from './types';

function url(conn: RoomConnection, path: string) {
  const pass = conn.current;
  return `${pass.api_url}${path}${path.includes('?') ? '&' : '?'}token=${encodeURIComponent(pass.token)}`;
}

async function failure(response: Response) {
  try {
    const body = (await response.json()) as { message?: string };
    if (body.message) return new Error(body.message);
  } catch {
    // Not JSON; fall through.
  }
  return new Error(response.status === 413 ? 'That upload is too large' : 'That did not work. Try again.');
}

export async function fetchMe(conn: RoomConnection): Promise<Me> {
  const response = await fetch(url(conn, '/me'));
  if (!response.ok) throw await failure(response);
  return (await response.json()) as Me;
}

export async function uploadSong(
  conn: RoomConnection,
  files: File[],
  fields: { name: string; artist: string; rights: boolean },
): Promise<{ id: string; song: SongInfo; skipped: string[] }> {
  const form = new FormData();
  form.set('name', fields.name);
  form.set('artist', fields.artist);
  form.set('rights', fields.rights ? 'accepted' : 'no');
  for (const file of files) form.append('file', file, file.name);
  const response = await fetch(url(conn, '/uploads'), { method: 'POST', body: form });
  if (!response.ok) throw await failure(response);
  return await response.json();
}

export async function deleteSong(conn: RoomConnection, id: string) {
  const response = await fetch(url(conn, `/uploads/${encodeURIComponent(id)}`), { method: 'DELETE' });
  if (!response.ok) throw await failure(response);
}

export async function reportSong(conn: RoomConnection, id: string, reason: ReportReason, details: string) {
  const response = await fetch(url(conn, `/songs/${encodeURIComponent(id)}/report`), {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ reason, details }),
  });
  if (!response.ok) throw await failure(response);
}
