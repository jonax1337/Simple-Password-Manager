"use client";

import { Checkbox } from "@/components/ui/checkbox"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
  ContextMenuSeparator,
} from "@/components/ui/context-menu";
import { Edit, User, Key, Trash2, ExternalLink, Star, GripVertical, Copy } from "lucide-react";
import { DynamicIcon } from "@/components/IconPicker";
import { updateEntry } from "@/lib/tauri";
import { useDraggable } from "@dnd-kit/core";
import { useToast } from "@/components/ui/use-toast";
import type { EntryData } from "@/lib/tauri";
import type { ColumnConfig } from "./types";

interface EntryListItemProps {
  entry: EntryData;
  visibleColumns: ColumnConfig[];
  isSelected: boolean;
  isContextMenuOpen: boolean;
  isChecked: boolean;
  isDragging?: boolean;
  onSelect: () => void;
  onToggleCheck: () => void;
  onContextMenuChange: (open: boolean) => void;
  onCopyField: (text: string, fieldName: string) => void;
  onOpenUrl: (url: string) => void;
  addToHistory?: (action: string, undo: () => Promise<void>, redo: () => Promise<void>) => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onRefresh: () => void;
  formatTimestamp: (timestamp?: string) => string;
}

export function EntryListItem({
  entry,
  visibleColumns,
  isSelected,
  isContextMenuOpen,
  isChecked,
  isDragging: isDraggingProp,
  onSelect,
  onToggleCheck,
  onContextMenuChange,
  onCopyField,
  onOpenUrl,
  onDuplicate,
  onDelete,
  onRefresh,
  formatTimestamp,
}: EntryListItemProps) {
  const { toast } = useToast();
  const iconId = entry.icon_id ?? 0;

  const { attributes, listeners, setNodeRef, isDragging: isDraggingLocal } = useDraggable({
    id: `entry-${entry.uuid}`,
    data: { 
      type: 'entry',
      entry: entry,
    },
  });

  const isDragging = isDraggingProp || isDraggingLocal;

  const handleToggleFavorite = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const wasFavorite = entry.is_favorite;
      const updatedEntry = {
        ...entry,
        is_favorite: !wasFavorite,
      };
      
      await updateEntry(updatedEntry);
      
      // Track for undo/redo
      if (onRefresh) {
        const entryUuid = entry.uuid;
        const originalState = wasFavorite;
        const newState = !wasFavorite;
        
        if ((window as any).__addToHistory) {
          (window as any).__addToHistory(
            `${newState ? 'Add' : 'Remove'} "${entry.title}" ${newState ? 'to' : 'from'} favorites`,
            async () => {
              await updateEntry({ ...entry, is_favorite: originalState });
              onRefresh();
            },
            async () => {
              await updateEntry({ ...entry, is_favorite: newState });
              onRefresh();
            }
          );
        }
      }
      
      onRefresh();
      toast({
        title: wasFavorite ? "Removed from favorites" : "Added to favorites",
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to update favorite status",
        variant: "destructive",
      });
    }
  };

  return (
    <ContextMenu onOpenChange={onContextMenuChange}>
      <ContextMenuTrigger onContextMenu={(e) => e.stopPropagation()}>
        <div
          ref={setNodeRef}
          className={`relative flex items-center border-b hover:bg-accent cursor-pointer ${
            isSelected || isContextMenuOpen || isChecked ? "bg-accent" : ""
          } ${isDragging ? "opacity-50 bg-accent" : ""}`}
          onDoubleClick={onSelect}
          style={{ touchAction: 'none' }}
        >
          {/* Fixed left section */}
          <div className="flex items-center gap-1 px-4 py-2.5 flex-shrink-0">
            {/* Drag Handle */}
            <div
              {...listeners}
              {...attributes}
              className="flex items-center justify-center w-4 cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground"
              onClick={(e) => e.stopPropagation()}
            >
              <GripVertical className="h-4 w-4" />
            </div>
            {/* Checkbox */}
            <div className="flex items-center w-8 justify-center">
              <Checkbox
                checked={isChecked}
                onCheckedChange={onToggleCheck}
                onClick={(e) => e.stopPropagation()}
              />
            </div>
            {/* Icon */}
            <div className="flex items-center w-8">
              <DynamicIcon iconId={iconId} className="h-4 w-4 text-muted-foreground" />
            </div>
          </div>
          
          {/* Scrollable columns section */}
          <div className="row-scroll flex-1 overflow-x-auto scrollbar-thin">
            <div className="flex h-full">
              {visibleColumns.map((col) => (
                <div 
                  key={col.id} 
                  className="relative overflow-hidden min-w-0 flex-shrink-0" 
                  style={{ width: `${col.width}px` }}
                >
                  <div className="py-2.5 h-full flex items-center">
                    {col.id === 'title' && (
                      <p className="truncate text-sm font-medium px-1">{entry.title}</p>
                    )}
                    {col.id === 'username' && (
                      <p 
                        className="truncate text-sm text-muted-foreground cursor-pointer px-1"
                        onDoubleClick={(e) => {
                          e.stopPropagation();
                          onCopyField(entry.username, "Username");
                        }}
                        title="Double-click to copy"
                      >
                        {entry.username || "—"}
                      </p>
                    )}
                    {col.id === 'password' && (
                      <p 
                        className="truncate text-sm text-muted-foreground font-mono cursor-pointer px-1"
                        onDoubleClick={(e) => {
                          e.stopPropagation();
                          onCopyField(entry.password, "Password");
                        }}
                        title="Double-click to copy"
                      >
                        {entry.password ? "••••••••" : "—"}
                      </p>
                    )}
                    {col.id === 'url' && (
                      <div className="flex items-center gap-1 px-1">
                        {entry.url ? (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              onOpenUrl(entry.url);
                            }}
                            className="flex items-center gap-1 text-sm text-blue-600 dark:text-blue-400 hover:underline truncate group"
                            title={entry.url}
                          >
                            <span className="truncate">{entry.url}</span>
                            <ExternalLink className="h-3 w-3 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity" />
                          </button>
                        ) : (
                          <span className="text-sm text-muted-foreground">—</span>
                        )}
                      </div>
                    )}
                    {col.id === 'notes' && (
                      <p className="truncate text-sm text-muted-foreground px-1" title={entry.notes}>
                        {entry.notes || "—"}
                      </p>
                    )}
                    {col.id === 'created' && (
                      <p className="truncate text-sm text-muted-foreground px-1">
                        {formatTimestamp(entry.created)}
                      </p>
                    )}
                    {col.id === 'modified' && (
                      <p className="truncate text-sm text-muted-foreground px-1">
                        {formatTimestamp(entry.modified)}
                      </p>
                    )}
                  </div>
                  {/* Resize handle visual separator - fills entire row height */}
                  <div className="absolute right-0 top-0 bottom-0 w-1 flex items-center justify-center pointer-events-none">
                    <div className="w-px h-full bg-border"></div>
                  </div>
                </div>
              ))}
            </div>
          </div>
          
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={onSelect}>
          <Edit className="mr-2 h-4 w-4" />
          Edit Entry
        </ContextMenuItem>
        <ContextMenuItem onClick={onDuplicate}>
          <Copy className="mr-2 h-4 w-4" />
          Duplicate Entry
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={handleToggleFavorite}>
          <Star 
            className={`mr-2 h-4 w-4 ${
              entry.is_favorite 
                ? "fill-yellow-400 text-yellow-400" 
                : "text-muted-foreground"
            }`}
          />
          {entry.is_favorite ? "Remove from Favorites" : "Add to Favorites"}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem 
          onClick={() => onCopyField(entry.username, "Username")}
          disabled={!entry.username}
        >
          <User className="mr-2 h-4 w-4" />
          Copy Username
        </ContextMenuItem>
        <ContextMenuItem 
          onClick={() => onCopyField(entry.password, "Password")}
          disabled={!entry.password}
        >
          <Key className="mr-2 h-4 w-4" />
          Copy Password
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem
          className="text-red-600 focus:text-red-600"
          onClick={onDelete}
        >
          <Trash2 className="mr-2 h-4 w-4" />
          Delete Entry
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
