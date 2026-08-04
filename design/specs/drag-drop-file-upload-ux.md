# Drag-and-Drop File Upload UX Spec

## Overview
A shared drag-and-drop file upload component used across contribution submission flows (`IssueDetailPage.tsx`) and document-upload flows (`TaxDocumentsTab.tsx`). This component provides a comprehensive set of states from idle to error handling, ensuring an accessible and robust user experience.

## Drop-Zone Anatomy
The upload drop-zone consists of the following key elements:
1. **Container:** A block-level area with a dashed border to indicate a drop target.
2. **Icon:** A primary visual indicator (e.g., an upload cloud or document icon).
3. **Copy:** Primary action text reading "Drag file or click to browse".
4. **Action Button:** A visible "Browse files" `<button>` which triggers the native file selection dialog.

## State Matrix

### 1. Idle
- **Visuals:** Standard dashed border using `border.default` color. Icon and copy are in default neutral text colors.
- **Interaction:** Awaits drag events or clicks on the "Browse files" button.

### 2. Drag-over
- **Visuals:** Dashed border changes to a solid or heavier dashed line using `interactive.focusRing` color. The background color tints to indicate an active drop area.
- **Interaction:** Actively indicates that a file being dragged is accepted if dropped.

### 3. Uploading
- **Visuals:** The drop-zone may collapse or remain visible to accept more files.
- **Queue Behavior (Multi-file):** Each file appears below the drop zone as a progress row.
- **Progress Row Anatomy:**
  - Filename text.
  - Progress bar filling horizontally.
  - "Cancel" action icon/button to abort the upload.

### 4. Paused
- **Visuals:** The progress bar pauses its animation and changes color to a neutral/paused state.
- **Action:** A "Resume" button becomes available to continue the upload.

### 5. Success
- **Visuals:** The progress bar reaches 100% and transitions to `semantic.success` color (`#22c55e`). A success icon (e.g., checkmark) appears next to the filename.
- **Action:** Option to remove the successfully uploaded file if it was a mistake.

### 6. File-Removed
- **Visuals:** The file row gracefully animates out of the list (fade and slide up).
- **Interaction:** The queue updates its layout without the removed file.

### 7. Error (Per Error Type)
Errors are displayed inline within the file's progress row.
- **Colors:** Error text and icon must use `#d32f2f` (light mode / `accessibility.color.semantic.error.value`) and `#ef4444` (dark mode / `darkMode.semantic.error`) to meet contrast requirements against their respective backgrounds. A color-only indicator is insufficient; a specific icon (e.g., alert triangle) must accompany the text.
- **Error Cases:**
  - **File-too-large:** "File exceeds the maximum size limit of X MB."
  - **Unsupported-type:** "This file type is not supported. Please upload a PDF, PNG, or JPG."
  - **Upload-failed-retry:** "Upload failed. [Retry button]"

## Accessibility Annotations
- **Keyboard & Operability:** The drag-and-drop functionality is treated as a progressive enhancement. The drop-zone must be fully reachable and operable via keyboard using the visible "Browse files" button. Users must be able to select files natively without needing a pointer device.
- **Screen Readers & Live Regions:** Upload progress and state changes (success, error) must be announced to assistive technologies using `aria-live="polite"`. Critical errors that require immediate attention should use `aria-live="assertive"` or `role="alert"`.

## Design QA & Validation Notes
1. **Contrast Compliance (WCAG 2.1 AA):** Verify that all drop-zone copy and error-message text meet the 4.5:1 contrast ratio in both light and dark themes. The designated error colors (`#d32f2f` and `#ef4444`) have been selected from `/design-tokens.json` to ensure compliance.
2. **Keyboard-Only Walkthrough:** Confirm that file selection, cancellation of uploads, and retry actions are fully achievable via the Tab and Enter/Space keys.
3. **Responsive Review:** Ensure the drop-zone and multi-file progress rows remain usable and visually intact at a viewport width of 375px.
