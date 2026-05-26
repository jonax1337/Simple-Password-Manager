"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuCheckboxItem,
  ContextMenuTrigger,
  ContextMenuSeparator,
} from "@/components/ui/context-menu";
import { ArrowUp, ArrowDown, ArrowUpDown } from "lucide-react";
import type { ColumnConfig, ColumnId, SortConfig } from "./types";
import type { EntryData } from "@/lib/tauri";

interface ColumnHeaderProps {
  columns: ColumnConfig[];
  visibleColumns: ColumnConfig[];
  currentSort: SortConfig;
  isAllSelected: boolean;
  entries: EntryData[];
  onToggleSelectAll: () => void;
  onSort: (columnId: ColumnId) => void;
  onToggleColumn: (columnId: ColumnId) => void;
  onColumnResize: (columnId: ColumnId, width: number) => void;
}

export function ColumnHeader({
  columns,
  visibleColumns,
  currentSort,
  isAllSelected,
  entries,
  onToggleSelectAll,
  onSort,
  onToggleColumn,
  onColumnResize,
}: ColumnHeaderProps) {
  const [resizingColumn, setResizingColumn] = useState<ColumnId | null>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);
  const visibleColumnsRef = useRef<ColumnConfig[]>(visibleColumns);

  useEffect(() => {
    // Keep visibleColumns ref in sync so resize calculations use latest column data
    visibleColumnsRef.current = visibleColumns;
  }, [visibleColumns]);

  const handleResizeStart = useCallback((e: React.MouseEvent, columnId: ColumnId, currentWidth: number) => {
    e.preventDefault();
    e.stopPropagation();
    setResizingColumn(columnId);
    startXRef.current = e.clientX;
    startWidthRef.current = currentWidth;
  }, []);

  const handleResizeMove = useCallback((e: MouseEvent) => {
    if (!resizingColumn || !scrollContainerRef.current) return;

    const diff = e.clientX - startXRef.current;
    let newWidth = Math.max(60, startWidthRef.current + diff);

    // Calculate available width (container width minus other columns)
    const containerWidth = scrollContainerRef.current.offsetWidth;
    const otherColumnsWidth = visibleColumnsRef.current
      .filter(col => col.id !== resizingColumn)
      .reduce((sum, col) => sum + col.width, 0);
    const maxWidth = containerWidth - otherColumnsWidth; // Use full available space

    // Limit width to available space only when there's at least enough room
    // for the minimum column width. If not, allow the column to grow beyond
    // the container, enabling horizontal scrolling instead of collapsing.
    if (maxWidth > 60) {
      newWidth = Math.min(newWidth, maxWidth);
    }

    onColumnResize(resizingColumn, newWidth);
  }, [resizingColumn, onColumnResize]);

  const handleResizeEnd = useCallback(() => {
    setResizingColumn(null);
  }, []);
  
  // Auto-resize column to fit content on double-click
  const handleAutoResize = useCallback((columnId: ColumnId) => {
    // Password column: always show dots (••••••••), fixed optimal width
    if (columnId === 'password') {
      onColumnResize(columnId, 120);
      return;
    }

    // Timestamps: fixed format length
    if (columnId === 'created' || columnId === 'modified') {
      onColumnResize(columnId, 160);
      return;
    }

    // Calculate optimal width based on content and header label
    let maxContentLength = 0;

    entries.forEach((entry) => {
      let content = "";
      switch (columnId) {
        case "title":
          content = entry.title;
          break;
        case "username":
          content = entry.username;
          break;
        case "url":
          content = entry.url;
          break;
        case "notes":
          content = entry.notes;
          break;
      }
      maxContentLength = Math.max(maxContentLength, content.length);
    });

    const columnConfig = columns.find((c) => c.id === columnId);
    const headerLabel = columnConfig?.label ?? "";
    const headerLength = headerLabel.length;

    const effectiveCharCount = Math.max(maxContentLength, headerLength);
    const basePadding = 32; // cell padding
    const sortIconWidth = 16; // space for sort icon

    // Estimate width: ~8px per character + padding + sort icon space
    const estimatedWidth = Math.max(
      80,
      Math.min(400, effectiveCharCount * 8 + basePadding + sortIconWidth),
    );
    onColumnResize(columnId, estimatedWidth);
  }, [columns, entries, onColumnResize]);

  // Add/remove event listeners for resize
  useEffect(() => {
    if (resizingColumn) {
      document.addEventListener('mousemove', handleResizeMove);
      document.addEventListener('mouseup', handleResizeEnd);
      return () => {
        document.removeEventListener('mousemove', handleResizeMove);
        document.removeEventListener('mouseup', handleResizeEnd);
      };
    }
  }, [resizingColumn, handleResizeMove, handleResizeEnd]);
  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div className="sticky top-0 z-10 flex items-center border-b bg-muted/50 text-xs font-semibold text-muted-foreground">
          {/* Fixed left section */}
          <div className="flex items-center gap-1 px-4 py-2 shrink-0">
            <div className="w-4"></div>
            <div className="w-8 flex items-center justify-center">
              <Checkbox
                checked={isAllSelected}
                onCheckedChange={onToggleSelectAll}
                title="Select all"
              />
            </div>
            <div className="w-8"></div>
          </div>
          
          {/* Scrollable columns section */}
          <div ref={scrollContainerRef} className="row-scroll flex-1 overflow-x-auto scrollbar-thin">
            <div className="flex h-full">
              {visibleColumns.map((col, index) => (
                <div
                  key={col.id}
                  className="relative flex items-center py-2"
                  style={{ width: `${col.width}px`, flexShrink: 0 }}
                >
                  <button
                    onClick={() => onSort(col.id)}
                    className="flex-1 flex items-center gap-1 hover:text-foreground transition-colors text-left overflow-hidden px-1"
                  >
                    <span className="truncate">{col.label}</span>
                    {currentSort.column === col.id ? (
                      currentSort.direction === 'asc' ? (
                        <ArrowUp className="h-3 w-3 shrink-0" />
                      ) : (
                        <ArrowDown className="h-3 w-3 shrink-0" />
                      )
                    ) : (
                      <ArrowUpDown className="h-3 w-3 opacity-30 shrink-0" />
                    )}
                  </button>
                  <div
                    className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/20 group flex items-center justify-center"
                    onMouseDown={(e) => handleResizeStart(e, col.id, col.width)}
                    onDoubleClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleAutoResize(col.id);
                    }}
                    title="Double-click to auto-fit"
                  >
                    <div className="w-px h-full bg-border group-hover:bg-primary transition-colors"></div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground">Show Columns</div>
        <ContextMenuSeparator />
        {columns.map((col) => (
          <ContextMenuCheckboxItem
            key={col.id}
            checked={col.visible}
            onCheckedChange={() => onToggleColumn(col.id)}
          >
            {col.label}
          </ContextMenuCheckboxItem>
        ))}
      </ContextMenuContent>
    </ContextMenu>
  );
}
