import { WebviewWindow, getAllWebviewWindows } from "@tauri-apps/api/webviewWindow";
import type { EntryData } from "@/lib/tauri";

// Labels for child windows that should be closed when main app closes/logs out
const CHILD_WINDOW_PREFIXES = ['entry-', 'settings', 'about'];

/**
 * Check if there are any open child windows (entry editors, settings)
 */
export async function hasOpenChildWindows(): Promise<boolean> {
  try {
    const allWindows = await getAllWebviewWindows();
    return allWindows.some(win => 
      CHILD_WINDOW_PREFIXES.some(prefix => win.label.startsWith(prefix))
    );
  } catch (error) {
    console.error("Failed to check child windows:", error);
    return false;
  }
}

/**
 * Request all child windows to close. Uses close() instead of destroy()
 * so that windows with unsaved changes can prompt the user first.
 * Returns true if all windows were closed, false if any remained open.
 */
export async function requestCloseAllChildWindows(): Promise<boolean> {
  try {
    const allWindows = await getAllWebviewWindows();
    
    const childWindows = allWindows.filter(win => 
      CHILD_WINDOW_PREFIXES.some(prefix => win.label.startsWith(prefix))
    );
    
    if (childWindows.length === 0) {
      return true;
    }
    
    // Request each window to close (triggers onCloseRequested which handles unsaved changes)
    for (const win of childWindows) {
      try {
        await win.close();
        console.log(`Requested close for child window: ${win.label}`);
      } catch (error) {
        // Ignore "window not found" errors - the window was already closed
        const errorMessage = typeof error === 'string' ? error : (error as Error)?.message || '';
        if (errorMessage.includes('window not found')) {
          console.log(`Window ${win.label} was already closed`);
        } else {
          console.error(`Failed to request close for window ${win.label}:`, error);
        }
      }
    }
    
    // Wait a moment for windows to process close requests and dialogs
    await new Promise(resolve => setTimeout(resolve, 100));
    
    // Check if all child windows are actually closed
    const stillOpen = await hasOpenChildWindows();
    return !stillOpen;
  } catch (error) {
    console.error("Failed to close child windows:", error);
    return false;
  }
}

/**
 * Force close all child windows without prompting (use destroy)
 */
export async function forceCloseAllChildWindows(): Promise<void> {
  try {
    const allWindows = await getAllWebviewWindows();
    
    for (const win of allWindows) {
      const label = win.label;
      const isChildWindow = CHILD_WINDOW_PREFIXES.some(prefix => label.startsWith(prefix));
      
      if (isChildWindow) {
        try {
          await win.destroy();
          console.log(`Force closed child window: ${label}`);
        } catch (error) {
          console.error(`Failed to force close window ${label}:`, error);
        }
      }
    }
  } catch (error) {
    console.error("Failed to force close child windows:", error);
  }
}

export async function openEntryWindow(entry: EntryData, groupUuid: string) {
  try {
    // Create a unique window label (must be alphanumeric and hyphens only)
    const windowLabel = `entry-${entry.uuid.replace(/[^a-zA-Z0-9-]/g, '-')}`;
    
    // Check if window already exists
    const existingWindow = await WebviewWindow.getByLabel(windowLabel);
    if (existingWindow) {
      await existingWindow.setFocus();
      return;
    }

    // Create new window with localhost URL for development
    // Pass group UUID as query parameter since sessionStorage doesn't work across Tauri windows
    const isDev = process.env.NODE_ENV === 'development';
    const baseUrl = isDev ? 'http://localhost:3000' : window.location.origin;
    
    const webview = new WebviewWindow(windowLabel, {
      url: `${baseUrl}/entry?uuid=${entry.uuid}&groupUuid=${encodeURIComponent(groupUuid)}`,
      title: "Edit Entry",
      width: 500,
      height: 700,
      resizable: false,
      maximizable: false,
      decorations: false,
      center: true,
    });

    console.log('Creating entry window:', windowLabel);
    
    // Wait for window to be ready
    webview.once("tauri://created", () => {
      console.log("Entry window created successfully");
    });

    webview.once("tauri://error", (e) => {
      console.error("Error creating entry window:", e);
    });
  } catch (error) {
    console.error("Failed to open entry window:", error);
    throw error;
  }
}

export async function openSettingsWindow() {
  try {
    const windowLabel = "settings";
    
    // Check if window already exists
    const existingWindow = await WebviewWindow.getByLabel(windowLabel);
    if (existingWindow) {
      await existingWindow.setFocus();
      return;
    }

    const isDev = process.env.NODE_ENV === 'development';
    const baseUrl = isDev ? 'http://localhost:3000' : window.location.origin;
    
    const webview = new WebviewWindow(windowLabel, {
      url: `${baseUrl}/settings`,
      title: "Settings",
      width: 600,
      height: 600,
      resizable: false,
      maximizable: false,
      decorations: false,
      center: true,
    });

    console.log('Creating settings window');
    
    webview.once("tauri://created", () => {
      console.log("Settings window created successfully");
    });

    webview.once("tauri://error", (e) => {
      console.error("Error creating settings window:", e);
    });
  } catch (error) {
    console.error("Failed to open settings window:", error);
    throw error;
  }
}

export async function openAboutWindow() {
  try {
    const windowLabel = "about";
    
    // Check if window already exists
    const existingWindow = await WebviewWindow.getByLabel(windowLabel);
    if (existingWindow) {
      await existingWindow.setFocus();
      return;
    }

    const isDev = process.env.NODE_ENV === 'development';
    const baseUrl = isDev ? 'http://localhost:3000' : window.location.origin;
    
    const webview = new WebviewWindow(windowLabel, {
      url: `${baseUrl}/about`,
      title: "About",
      width: 600,
      height: 700,
      resizable: false,
      maximizable: false,
      decorations: false,
      center: true,
    });

    console.log('Creating about window');
    
    webview.once("tauri://created", () => {
      console.log("About window created successfully");
    });

    webview.once("tauri://error", (e) => {
      console.error("Error creating about window:", e);
    });
  } catch (error) {
    console.error("Failed to open about window:", error);
    throw error;
  }
}
