export const POLL_INTERVAL_MS = 1100;
export const BASE_PLAYBACK_INTERVAL_MS = 700;
export const PLAYBACK_SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2];

export const NODE_DIMENSIONS = {
  width: 172,
  height: 60
};

export const KIND_CONFIG = {
  function: {
    label: "Function",
    icon: "f",
    badgeClass: "is-function"
  },
  type: {
    label: "Data",
    icon: "db",
    badgeClass: "is-data"
  },
  test: {
    label: "Test",
    icon: "t",
    badgeClass: "is-test"
  }
};

export const EDGE_COLORS = {
  control_flow: "#6c6c6c",
  data_flow: "#555555",
  test_link: "#6c6c6c"
};

export const EVENT_LABEL = {
  function_enter: "enter",
  function_exit: "exit",
  value_snapshot: "snapshot",
  test_started: "test start",
  test_passed: "test pass",
  test_failed: "test fail"
};

export const DEFAULT_SERVER_STATUS = {
  running: false,
  can_rerun: false,
  generation: 0,
  session_title: "",
  last_error: null
};
