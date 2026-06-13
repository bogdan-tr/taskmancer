export type TaskStatus = "backlog" | "do" | "in-progress" | "blocked" | "done";

export type Priority = "low" | "medium" | "high";

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  project?: string;
  tags: string[];
  priority: Priority;
  due?: string;
  scheduled?: string;
  order: number;
  created: string;
  depends_on: string[];
  notes: string;
}

export const TASK_STATUSES: TaskStatus[] = ["backlog", "do", "in-progress", "blocked", "done"];

export const STATUS_LABELS: Record<TaskStatus, string> = {
  backlog: "Backlog",
  do: "Do",
  "in-progress": "In Progress",
  blocked: "Blocked",
  done: "Done",
};

export const PRIORITIES: Priority[] = ["low", "medium", "high"];

export const PRIORITY_LABELS: Record<Priority, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
};

export interface Project {
  id: string;
  name: string;
  color: string;
  order: number;
  created: string;
}

/** Matches `DEFAULT_PROJECT_COLOR` in `src-tauri/src/project.rs`. */
export const DEFAULT_PROJECT_COLOR = "#3b82f6";
