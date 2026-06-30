import { listSavedViews } from "./api";
import type { SavedView } from "./types";

class SavedViewsState {
  items: SavedView[] = $state([]);
}

export const savedViewsState = new SavedViewsState();

export async function refreshSavedViews(): Promise<void> {
  try {
    savedViewsState.items = await listSavedViews();
  } catch {
    savedViewsState.items = [];
  }
}
