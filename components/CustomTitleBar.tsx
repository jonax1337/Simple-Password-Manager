"use client";

import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X, Copy, Save, LogOut, Undo2, Redo2, Clipboard, ClipboardPaste, FolderOpen, Database as DatabaseIcon, Eye, EyeOff, Info, ExternalLink, FileText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Settings } from "@/components/animate-ui/icons/settings";
import { Search } from "@/components/animate-ui/icons/search";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
} from "@/components/ui/dropdown-menu";
import { openSettingsWindow } from "@/lib/window";
import { getRecentDatabases } from "@/lib/storage";
import { openDatabaseInNewInstance } from "@/lib/tauri";

interface CustomTitleBarProps {
  title?: string;
  hideMaximize?: boolean;
  showMenu?: boolean;
  isDirty?: boolean;
  onSave?: () => void;
  onLogout?: () => void;
  onToggleSearch?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  onNewDatabase?: () => void;
  onTogglePasswords?: () => void;
  onAbout?: () => void;
  canUndo?: boolean;
  canRedo?: boolean;
  passwordsVisible?: boolean;
}

export function CustomTitleBar({ 
  title = "Password Manager",
  hideMaximize = false,
  showMenu = false,
  isDirty = false,
  onSave,
  onLogout,
  onToggleSearch,
  onUndo,
  onRedo,
  onCopy,
  onPaste,
  onNewDatabase,
  onTogglePasswords,
  onAbout,
  canUndo = false,
  canRedo = false,
  passwordsVisible = false,
}: CustomTitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false);
  const [isSettingsHovered, setIsSettingsHovered] = useState(false);
  const [isSearchHovered, setIsSearchHovered] = useState(false);
  const [recentDatabases, setRecentDatabases] = useState<string[]>([]);

  useEffect(() => {
    setRecentDatabases(getRecentDatabases());
  }, []);

  const handleOpenDatabase = async (dbPath: string) => {
    try {
      await openDatabaseInNewInstance(dbPath);
    } catch (error) {
      console.error('Failed to open database in new instance:', error);
      alert(`Could not open database: ${error}`);
    }
  };

  useEffect(() => {
    const checkMaximized = async () => {
      const appWindow = getCurrentWindow();
      const maximized = await appWindow.isMaximized();
      setIsMaximized(maximized);
    };

    checkMaximized();

    const unlisten = getCurrentWindow().onResized(() => {
      checkMaximized();
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  const handleMinimize = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.minimize();
  };

  const handleMaximize = async () => {
    const appWindow = getCurrentWindow();
    if (isMaximized) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
    setIsMaximized(!isMaximized);
  };

  const handleClose = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  };

  const startDragging = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.startDragging();
  };

  // Unified title bar layout: Icon left --- Title centered --- Window Controls right
  return (
    <div 
      className="h-9 bg-muted/50 border-b select-none flex items-center justify-between pl-2 relative"
      data-tauri-drag-region
      onMouseDown={(e) => {
        if (e.target === e.currentTarget || (e.target as HTMLElement).hasAttribute('data-tauri-drag-region')) {
          startDragging();
        }
      }}
    >
      {/* Left: Icon + Optional Menus (only for main app) */}
      <div className="flex items-center gap-1 z-10">
        <img 
          src="/app-icon.png" 
          alt="App Icon" 
          className="h-4 w-4 ml-1"
          data-tauri-drag-region
        />
        {showMenu && (
          <>
            {/* File Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button className="px-2 py-1 text-xs font-medium hover:bg-accent rounded transition-colors">
                  File
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-52">
                <DropdownMenuItem
                  onClick={onNewDatabase}
                  className="gap-2 cursor-pointer"
                >
                  <DatabaseIcon className="h-4 w-4" />
                  <span>New Database</span>
                  <span className="ml-auto text-xs text-muted-foreground">Ctrl+N</span>
                </DropdownMenuItem>
                <DropdownMenuSub>
                  <DropdownMenuSubTrigger className="gap-2">
                    <FolderOpen className="h-4 w-4" />
                    <span>Open Recent</span>
                  </DropdownMenuSubTrigger>
                  <DropdownMenuSubContent>
                    {recentDatabases.length > 0 ? (
                      recentDatabases.map((dbPath) => (
                        <DropdownMenuItem
                          key={dbPath}
                          onClick={() => handleOpenDatabase(dbPath)}
                          className="gap-2 cursor-pointer"
                        >
                          <DatabaseIcon className="h-4 w-4" />
                          <span className="truncate max-w-[200px]" title={dbPath}>
                            {dbPath.split(/[/\\]/).pop() || dbPath}
                          </span>
                        </DropdownMenuItem>
                      ))
                    ) : (
                      <DropdownMenuItem disabled className="text-muted-foreground">
                        No recent databases
                      </DropdownMenuItem>
                    )}
                  </DropdownMenuSubContent>
                </DropdownMenuSub>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={onSave}
                  disabled={!isDirty}
                  className="gap-2 cursor-pointer"
                >
                  <Save className="h-4 w-4" />
                  <span>Save Database</span>
                  <span className="ml-auto text-xs text-muted-foreground">Ctrl+S</span>
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={onLogout}
                  className="gap-2 cursor-pointer"
                >
                  <LogOut className="h-4 w-4" />
                  <span>Close Database</span>
                  <span className="ml-auto text-xs text-muted-foreground">Ctrl+W</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Edit Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button className="px-2 py-1 text-xs font-medium hover:bg-accent rounded transition-colors">
                  Edit
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-52">
                <DropdownMenuItem
                  onClick={onUndo}
                  disabled={!canUndo}
                  className="gap-2 cursor-pointer"
                >
                  <Undo2 className="h-4 w-4" />
                  <span>Undo</span>
                  <span className="ml-auto text-xs text-muted-foreground">Ctrl+Z</span>
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={onRedo}
                  disabled={!canRedo}
                  className="gap-2 cursor-pointer"
                >
                  <Redo2 className="h-4 w-4" />
                  <span>Redo</span>
                  <span className="ml-auto text-xs text-muted-foreground">Ctrl+Y</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* View Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button className="px-2 py-1 text-xs font-medium hover:bg-accent rounded transition-colors">
                  View
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-52">
                <DropdownMenuItem
                  onClick={onToggleSearch}
                  className="gap-2 cursor-pointer"
                >
                  <Search size={16} />
                  <span>Toggle Search</span>
                  <span className="ml-auto text-xs text-muted-foreground">Ctrl+F</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Help Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button className="px-2 py-1 text-xs font-medium hover:bg-accent rounded transition-colors">
                  Help
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-52">
                <DropdownMenuItem
                  onClick={onAbout}
                  className="gap-2 cursor-pointer"
                >
                  <Info className="h-4 w-4" />
                  <span>About</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </>
        )}
      </div>

      {/* Center: Title */}
      <div className="absolute left-1/2 -translate-x-1/2 flex items-center" data-tauri-drag-region>
        <span className="text-xs font-semibold text-foreground truncate max-w-md" data-tauri-drag-region>
          {title}
        </span>
      </div>

      {/* Right: Optional Search + Settings + Window Controls */}
      <div className="flex items-center gap-1 h-full z-10" data-tauri-drag-region>
        {showMenu && onToggleSearch && (
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={onToggleSearch}
            onMouseEnter={() => setIsSearchHovered(true)}
            onMouseLeave={() => setIsSearchHovered(false)}
            title="Toggle Search (Ctrl+F)"
          >
            <Search size={16} animate={isSearchHovered} animation="find"/>
          </Button>
        )}
        
        {showMenu && (
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => openSettingsWindow()}
            onMouseEnter={() => setIsSettingsHovered(true)}
            onMouseLeave={() => setIsSettingsHovered(false)}
            title="Settings"
          >
            <Settings size={16} animate={isSettingsHovered} />
          </Button>
        )}

        {showMenu && <div className="h-5 w-px bg-border mx-1" data-tauri-drag-region />}

        <button
          onClick={handleMinimize}
          className="h-full px-3 hover:bg-accent transition-colors flex items-center justify-center"
          aria-label="Minimize"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        {!hideMaximize && (
          <button
            onClick={handleMaximize}
            className="h-full px-3 hover:bg-accent transition-colors flex items-center justify-center"
            aria-label={isMaximized ? "Restore" : "Maximize"}
          >
            {isMaximized ? (
              <Copy className="h-3 w-3" />
            ) : (
              <Square className="h-3 w-3" />
            )}
          </button>
        )}
        <button
          onClick={handleClose}
          className="h-full px-3 hover:bg-destructive hover:text-destructive-foreground transition-colors flex items-center justify-center"
          aria-label="Close"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
