"use client";

import { useState, useEffect, useRef } from "react";
import { saveColumnConfig, getColumnConfig, type ColumnVisibility } from "@/lib/storage";
import { DEFAULT_COLUMNS, type ColumnConfig, type ColumnId, type SortConfig } from "../types";

export function useColumnConfig(databasePath?: string) {
  // Use lazy initialization to load columns from storage
  const [columns, setColumns] = useState<ColumnConfig[]>(() => {
    if (databasePath) {
      const savedConfig = getColumnConfig(databasePath);
      return DEFAULT_COLUMNS.map(col => ({
        ...col,
        visible: savedConfig ? (savedConfig[col.id as keyof ColumnVisibility] ?? col.visible) : col.visible
      }));
    }
    return DEFAULT_COLUMNS;
  });
  
  const previousWidthRef = useRef<number>(0);

  // Reload column config when database path changes
  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    if (databasePath) {
      const savedConfig = getColumnConfig(databasePath);
      setColumns(DEFAULT_COLUMNS.map(col => ({
        ...col,
        visible: savedConfig ? (savedConfig[col.id as keyof ColumnVisibility] ?? col.visible) : col.visible
      })));
    }
  }, [databasePath]);
  /* eslint-enable react-hooks/set-state-in-effect */

  // Toggle column visibility and save to storage
  const toggleColumn = (columnId: ColumnId) => {
    setColumns(prev => {
      const newColumns = prev.map(col => 
        col.id === columnId ? { ...col, visible: !col.visible } : col
      );
      
      // Save to storage if we have a database path
      if (databasePath) {
        const config: ColumnVisibility = {
          title: newColumns.find(c => c.id === 'title')?.visible ?? true,
          username: newColumns.find(c => c.id === 'username')?.visible ?? true,
          password: newColumns.find(c => c.id === 'password')?.visible ?? true,
          url: newColumns.find(c => c.id === 'url')?.visible ?? true,
          notes: newColumns.find(c => c.id === 'notes')?.visible ?? true,
          totp: newColumns.find(c => c.id === 'totp')?.visible ?? true,
          created: newColumns.find(c => c.id === 'created')?.visible ?? false,
          modified: newColumns.find(c => c.id === 'modified')?.visible ?? false,
        };
        saveColumnConfig(databasePath, config);
      }
      
      return newColumns;
    });
  };

  // Update column width (in-memory only, resets on DB reload)
  const updateColumnWidth = (columnId: ColumnId, width: number) => {
    setColumns(prev => 
      prev.map(col => 
        col.id === columnId ? { ...col, width } : col
      )
    );
  };
  
  // Adjust column widths proportionally on window resize
  useEffect(() => {
    const handleResize = () => {
      const currentWidth = window.innerWidth;
      
      // Only adjust if we have a previous width and it changed significantly
      if (previousWidthRef.current > 0 && Math.abs(currentWidth - previousWidthRef.current) > 50) {
        const ratio = currentWidth / previousWidthRef.current;
        
        setColumns(prev => prev.map(col => ({
          ...col,
          width: Math.max(60, col.width * ratio)
        })));
      }
      
      previousWidthRef.current = currentWidth;
    };
    
    // Set initial width
    previousWidthRef.current = window.innerWidth;
    
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  const visibleColumns = columns.filter(col => col.visible);

  return {
    columns,
    visibleColumns,
    toggleColumn,
    updateColumnWidth,
  };
}

export function useSortConfig(groupUuid: string) {
  const [sortConfigPerGroup, setSortConfigPerGroup] = useState<Record<string, SortConfig>>({});

  // Get current sort config for this group (default: created desc)
  const currentSort = sortConfigPerGroup[groupUuid] || { 
    column: 'created' as ColumnId, 
    direction: 'desc' as const 
  };

  // Set sorting for current group
  const handleSort = (columnId: ColumnId) => {
    setSortConfigPerGroup(prev => {
      const current = prev[groupUuid] || { column: 'created', direction: 'desc' };
      const newDirection = current.column === columnId && current.direction === 'asc' ? 'desc' : 'asc';
      return {
        ...prev,
        [groupUuid]: { column: columnId, direction: newDirection }
      };
    });
  };

  return {
    currentSort,
    handleSort,
  };
}
