import type {
  WorkEnvironmentChanged,
  WorkChanged,
  WorkHumanChanged,
  WorkDecisionPreferenceChanged,
  WorkModelsChanged,
  AccountChanged,
  PanelState,
  PageAssistChanged,
  ProfilesView,
  FaviconsView,
  NoteOpenRequested,
  NoteChanges,
  ResourceChanged,
  DownloadsChanged,
  BlockerStatusChanged,
  BrowserCredentialCapabilityChanged,
  FocusChanged,
  ExtensionActionFailed,
  ExtensionActionShortcut,
  ExtensionActionsChanged,
  FindResultView,
  ImportJobView,
  ItemsChanged,
  KeymapChanged,
  LayoutChanged,
  MediaPlayerChanged,
  OperationProcessed,
  PagePermissionPromptChanged,
  RuntimeStatusChanged,
  SearchChanged,
  TabChanged,
  UiCommand,
  VoiceChanged,
  VoiceLevels,
  SwipeScroll,
  WebExtensionAccessRequested,
} from "./bindings";

type PayloadEvent<T> = { payload: T };
type Listener<T> = (event: PayloadEvent<T>) => void;
type PresentationTab = { tab: TabChanged; active: string | null };

function scopedEvent<T>(name: string) {
  return {
    listen(listener: Listener<T>): Promise<() => void> {
      const handler = (event: Event) => listener({ payload: (event as CustomEvent<T>).detail });
      window.addEventListener(name, handler);
      return Promise.resolve(() => window.removeEventListener(name, handler));
    },
  };
}

// Rust delivers these directly into the intended WebView. Do not replace this
// with @tauri-apps/api/event: its listen command accepts a caller-selected
// target, which would let the launcher panel subscribe to main-window state.
export const nativeEventNames = {
  workEnvironmentChanged: "drift:work-environment-changed",
  workChanged: "drift:work-changed",
  workHumanChanged: "drift:work-human-changed",
  workDecisionPreferenceChanged: "drift:work-decision-preference-changed",
  workModelsChanged: "drift:work-models-changed",
  pageAssistChanged: "drift:page-assist-changed",
  accountChanged: "drift:account-changed",
  panelState: "drift:panel-state",
  favicons: "drift:favicons",
  noteOpenRequested: "drift:note-open-requested",
  resourceChanged: "drift:resource-changed",
  notesChanged: "drift:notes-changed",
  downloadsChanged: "drift:downloads-changed",
  resourceClose: "drift:resource-close",
  resourceCloseCancelled: "drift:resource-close-cancelled",
  browserCredentialCapabilityChanged: "drift:browser-credential-capability",
  itemsChanged: "drift:items",
  tabChanged: "drift:tab",
  extensionActionsChanged: "drift:extension-actions",
  extensionActionFailed: "drift:extension-action-failed",
  extensionActionShortcut: "drift:extension-action-shortcut",
  pagePermissionPromptChanged: "drift:page-permission-prompt",
  webExtensionAccessRequested: "drift:web-extension-access",
  webExtensionDropped: "drift:web-extension-dropped",
  browserReturn: "drift:browser-return",
  presentationTab: "drift:presentation-tab",
  uiCommand: "drift:ui-command",
  keymapChanged: "drift:keymap-changed",
  importProgress: "drift:import-progress",
  find: "drift:find",
  searchChanged: "drift:search",
  layoutChanged: "drift:layout",
  mediaPlayerChanged: "drift:media-player",
  operationProcessed: "drift:operation-processed",
  runtimeStatusChanged: "drift:runtime-status",
  blockerStatusChanged: "drift:blocker-status",
  focusChanged: "drift:focus",
  voiceChanged: "drift:voice-changed",
  voiceLevels: "drift:voice-levels",
  swipeScroll: "drift:swipe",
  profilesChanged: "drift:profiles",
} as const;

export const events = {
  workEnvironmentChanged: scopedEvent<WorkEnvironmentChanged>(
    nativeEventNames.workEnvironmentChanged,
  ),
  workChanged: scopedEvent<WorkChanged>(nativeEventNames.workChanged),
  workHumanChanged: scopedEvent<WorkHumanChanged>(nativeEventNames.workHumanChanged),
  workDecisionPreferenceChanged: scopedEvent<WorkDecisionPreferenceChanged>(
    nativeEventNames.workDecisionPreferenceChanged,
  ),
  workModelsChanged: scopedEvent<WorkModelsChanged>(nativeEventNames.workModelsChanged),
  pageAssistChanged: scopedEvent<PageAssistChanged>(nativeEventNames.pageAssistChanged),
  accountChanged: scopedEvent<AccountChanged>(nativeEventNames.accountChanged),
  favicons: scopedEvent<FaviconsView>(nativeEventNames.favicons),
  noteOpenRequested: scopedEvent<NoteOpenRequested>(nativeEventNames.noteOpenRequested),
  resourceClose: scopedEvent<string>(nativeEventNames.resourceClose),
  resourceCloseCancelled: scopedEvent<string>(nativeEventNames.resourceCloseCancelled),
  resourceChanged: scopedEvent<ResourceChanged>(nativeEventNames.resourceChanged),
  notesChanged: scopedEvent<NoteChanges>(nativeEventNames.notesChanged),
  downloadsChanged: scopedEvent<DownloadsChanged>(nativeEventNames.downloadsChanged),
  panelState: scopedEvent<PanelState>(nativeEventNames.panelState),
  browserCredentialCapabilityChanged: scopedEvent<BrowserCredentialCapabilityChanged>(
    nativeEventNames.browserCredentialCapabilityChanged,
  ),
  itemsChanged: scopedEvent<ItemsChanged>(nativeEventNames.itemsChanged),
  tabChanged: scopedEvent<TabChanged>(nativeEventNames.tabChanged),
  extensionActionsChanged: scopedEvent<ExtensionActionsChanged>(
    nativeEventNames.extensionActionsChanged,
  ),
  extensionActionFailed: scopedEvent<ExtensionActionFailed>(nativeEventNames.extensionActionFailed),
  extensionActionShortcut: scopedEvent<ExtensionActionShortcut>(
    nativeEventNames.extensionActionShortcut,
  ),
  pagePermissionPromptChanged: scopedEvent<PagePermissionPromptChanged>(
    nativeEventNames.pagePermissionPromptChanged,
  ),
  webExtensionAccessRequested: scopedEvent<WebExtensionAccessRequested>(
    nativeEventNames.webExtensionAccessRequested,
  ),
  /** The path of an extension package dropped on the browser. */
  webExtensionDropped: scopedEvent<string>(nativeEventNames.webExtensionDropped),
  browserReturn: scopedEvent<ItemsChanged>(nativeEventNames.browserReturn),
  presentationTab: scopedEvent<PresentationTab>(nativeEventNames.presentationTab),
  uiCommand: scopedEvent<UiCommand>(nativeEventNames.uiCommand),
  keymapChanged: scopedEvent<KeymapChanged>(nativeEventNames.keymapChanged),
  importProgress: scopedEvent<ImportJobView>(nativeEventNames.importProgress),
  find: scopedEvent<FindResultView>(nativeEventNames.find),
  searchChanged: scopedEvent<SearchChanged>(nativeEventNames.searchChanged),
  layoutChanged: scopedEvent<LayoutChanged>(nativeEventNames.layoutChanged),
  mediaPlayerChanged: scopedEvent<MediaPlayerChanged>(nativeEventNames.mediaPlayerChanged),
  operationProcessed: scopedEvent<OperationProcessed>(nativeEventNames.operationProcessed),
  runtimeStatusChanged: scopedEvent<RuntimeStatusChanged>(nativeEventNames.runtimeStatusChanged),
  blockerStatusChanged: scopedEvent<BlockerStatusChanged>(nativeEventNames.blockerStatusChanged),
  focusChanged: scopedEvent<FocusChanged>(nativeEventNames.focusChanged),
  voiceChanged: scopedEvent<VoiceChanged>(nativeEventNames.voiceChanged),
  voiceLevels: scopedEvent<VoiceLevels>(nativeEventNames.voiceLevels),
  swipeScroll: scopedEvent<SwipeScroll>(nativeEventNames.swipeScroll),
  profilesChanged: scopedEvent<ProfilesView>(nativeEventNames.profilesChanged),
};
