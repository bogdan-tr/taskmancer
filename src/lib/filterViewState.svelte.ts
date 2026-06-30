import type { FilterConfig, SortConfig } from "./types";

export const DEFAULT_FILTER_CONFIG: FilterConfig = {
  text: "",
  titleOnly: false,
  statuses: [],
  projectIds: [],
  tags: [],
  tagMode: "any",
  dueFilter: { type: "any" },
  scheduledFilter: { type: "any" },
  priorities: [],
  estimateMinHours: null,
  estimateMaxHours: null,
  trackedMinHours: null,
  trackedMaxHours: null,
  hasSubtasks: "any",
  isRecurring: "any",
  isArchived: "active_only",
  groupMode: "all",
};

export const DEFAULT_SORT_CONFIG: SortConfig = { levels: [] };

/** Reactive state for the Filter tab — shared across FilterView, FilterDrawer, and KanbanBoard. */
class FilterViewState {
  config: FilterConfig = $state({ ...DEFAULT_FILTER_CONFIG });
  sort: SortConfig = $state({ levels: [] });

  /** The id of the currently-loaded saved view, or `null` when the user
   *  has unsaved changes or is building a new view. */
  activeViewId: string | null = $state(null);

  /** Whether the slide-in filter drawer is open. */
  drawerOpen: boolean = $state(false);

  /** Set to `"filter"` by Sidebar when the user clicks a saved view; cleared
   *  by KanbanBoard after switching to the filter tab. */
  requestedView: "filter" | null = $state(null);

  reset() {
    this.config = { ...DEFAULT_FILTER_CONFIG };
    this.sort = { levels: [] };
    this.activeViewId = null;
  }
}

export const filterViewState = new FilterViewState();
