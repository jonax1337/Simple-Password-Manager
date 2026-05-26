"use client";

import { motion } from "framer-motion";
import { Button } from "@/components/ui/button"
import { ChevronRight, ChevronDown, Plus, Trash2, Edit2 } from "lucide-react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
  ContextMenuSeparator,
} from "@/components/ui/context-menu";
import { DynamicIcon } from "@/components/IconPicker";
import { useDraggable, useDroppable } from "@dnd-kit/core";
import type { DraggableFolderProps } from "./types";

export function DraggableFolder({
  group,
  depth,
  selectedUuid,
  expandedGroups,
  overId,
  activeType,
  onToggleExpand,
  onSelectGroup,
  onOpenCreateDialog,
  onOpenRenameDialog,
  onDeleteGroup,
}: DraggableFolderProps) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: group.uuid,
    disabled: depth === 0,
    data: { type: 'folder' },
  });
  
  const dragListeners = {
    ...listeners,
    onPointerDown: (e: React.PointerEvent) => {
      if (e.button === 2) return;
      listeners?.onPointerDown?.(e);
    },
  };

  const { setNodeRef: setDropRef } = useDroppable({
    id: group.uuid,
  });

  const isExpanded = expandedGroups.has(group.uuid);
  const isSelected = group.uuid === selectedUuid;
  const hasChildren = group.children && group.children.length > 0;
  const folderIconId = group.icon_id ?? 48;
  const isDropTarget = overId === group.uuid;

  return (
    <div key={group.uuid}>
      <div ref={setDropRef} data-group-id={group.uuid}>
        <div 
          ref={setNodeRef}
          {...dragListeners}
          {...attributes}
          style={{ 
            opacity: isDragging ? 0.6 : 1,
            cursor: isDragging ? 'grabbing' : 'default'
          }}
        >
          <ContextMenu>
            <ContextMenuTrigger asChild onContextMenu={(e: React.MouseEvent) => e.stopPropagation()}>
              <div
                className={`flex items-center gap-1 px-2 py-1.5 rounded transition-all outline-hidden hover:bg-accent/50 data-[state=open]:bg-accent/50 ${
                  isSelected ? "bg-accent font-medium" : ""
                } ${isDropTarget ? "bg-primary/20 ring-2 ring-primary ring-inset" : ""}`}
                style={{ paddingLeft: `${depth * 12 + 8}px` }}
              >
                <Button
                  variant="ghost"
                  size="icon"
                  className={`h-5 w-5 p-0 ${!hasChildren ? 'pointer-events-none hover:bg-transparent' : ''}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    hasChildren && onToggleExpand(group.uuid);
                  }}
                >
                  {hasChildren ? (
                    <motion.div
                      animate={{ rotate: isExpanded ? 90 : 0 }}
                      transition={{ duration: 0.2, ease: "easeInOut" }}
                    >
                      <ChevronRight className="h-3 w-3" />
                    </motion.div>
                  ) : (
                    <div className="h-3 w-3" />
                  )}
                </Button>

                <DynamicIcon iconId={folderIconId} className="h-4 w-4 text-muted-foreground" />

                <span
                  className="flex-1 text-sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    onSelectGroup(group.uuid);
                  }}
                  onDoubleClick={(e) => {
                    e.stopPropagation();
                    hasChildren && onToggleExpand(group.uuid);
                  }}
                >
                  {group.name}
                </span>
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem onClick={() => onOpenCreateDialog(group.uuid)}>
                <Plus className="mr-2 h-4 w-4" />
                New Subgroup
              </ContextMenuItem>
              <ContextMenuItem onClick={() => onOpenRenameDialog(group)}>
                <Edit2 className="mr-2 h-4 w-4" />
                Rename
              </ContextMenuItem>
              {depth > 0 && (
                <>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    onClick={() => onDeleteGroup(group.uuid)}
                    className="text-destructive"
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    Delete
                  </ContextMenuItem>
                </>
              )}
            </ContextMenuContent>
          </ContextMenu>
        </div>
      </div>

      {isExpanded && hasChildren && group.children.map((child) => (
        <DraggableFolder
          key={child.uuid}
          group={child}
          depth={depth + 1}
          selectedUuid={selectedUuid}
          expandedGroups={expandedGroups}
          overId={overId}
          activeType={activeType}
          onToggleExpand={onToggleExpand}
          onSelectGroup={onSelectGroup}
          onOpenCreateDialog={onOpenCreateDialog}
          onOpenRenameDialog={onOpenRenameDialog}
          onDeleteGroup={onDeleteGroup}
        />
      ))}
    </div>
  );
}
