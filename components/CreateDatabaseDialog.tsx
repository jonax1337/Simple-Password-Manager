"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { FolderOpen } from "lucide-react";
import { createDatabase, openDatabaseInNewInstance } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import { open } from "@tauri-apps/plugin-dialog";
import { saveLastDatabasePath, addRecentDatabase } from "@/lib/storage";
import { PasswordStrengthMeter } from "@/components/PasswordStrengthMeter";

interface CreateDatabaseDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: (openedInNewInstance: boolean) => void;
  hasOpenDatabase?: boolean;
}

export function CreateDatabaseDialog({
  isOpen,
  onClose,
  onSuccess,
  hasOpenDatabase = false,
}: CreateDatabaseDialogProps) {
  const [folderPath, setFolderPath] = useState("");
  const [fileName, setFileName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const { toast } = useToast();

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });

      if (selected) {
        setFolderPath(selected as string);
      }
    } catch (error: any) {
      toast({
        title: "Error",
        description: error?.toString() || "Failed to select folder",
        variant: "destructive",
      });
    }
  };

  const handleCreate = async () => {
    if (!folderPath || !fileName || !password) {
      toast({
        title: "Missing Information",
        description: "Please fill in all fields",
        variant: "destructive",
      });
      return;
    }

    if (password !== confirmPassword) {
      toast({
        title: "Password Mismatch",
        description: "Passwords do not match",
        variant: "destructive",
      });
      return;
    }

    if (password.length < 4) {
      toast({
        title: "Weak Password",
        description: "Password must be at least 4 characters",
        variant: "destructive",
      });
      return;
    }

    setLoading(true);
    try {
      const fileNameWithExt = fileName.endsWith(".kdbx")
        ? fileName
        : `${fileName}.kdbx`;
      const fullPath = `${folderPath}\\${fileNameWithExt}`;

      await createDatabase(fullPath, password);
      addRecentDatabase(fullPath);
      
      if (hasOpenDatabase) {
        // Open in new instance if a database is already open
        await openDatabaseInNewInstance(fullPath);
        toast({
          title: "Success",
          description: "Database created and opened in new window",
          variant: "success",
        });
        onSuccess(true);
      } else {
        // Open in current instance
        saveLastDatabasePath(fullPath);
        toast({
          title: "Success",
          description: "Database created successfully",
          variant: "success",
        });
        onSuccess(false);
      }
      
      setFolderPath("");
      setFileName("");
      setPassword("");
      setConfirmPassword("");
      onClose();
    } catch (error: any) {
      toast({
        title: "Failed to Create",
        description: error?.toString() || "Could not create database",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    if (!loading) {
      setFolderPath("");
      setFileName("");
      setPassword("");
      setConfirmPassword("");
      onClose();
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Create New Database</DialogTitle>
          <DialogDescription>
            Create a new KeePass database file. Choose a secure master password
            to protect your data.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="folder">Save Location</Label>
            <div className="flex gap-2">
              <Input
                id="folder"
                value={folderPath}
                placeholder="Select folder..."
                readOnly
                className="flex-1"
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={handleSelectFolder}
                className="h-10 w-10"
              >
                <FolderOpen className="h-5 w-5" />
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="filename">Database Name</Label>
            <Input
              id="filename"
              value={fileName}
              onChange={(e) => setFileName(e.target.value)}
              placeholder="MyDatabase"
            />
            <p className="text-xs text-muted-foreground">
              .kdbx extension will be added automatically
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-password">Master Password</Label>
            <Input
              id="new-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Choose a strong password"
            />
            <PasswordStrengthMeter password={password} />
          </div>

          <div className="space-y-2">
            <Label htmlFor="confirm-new-password">Confirm Password</Label>
            <Input
              id="confirm-new-password"
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleCreate()}
              placeholder="Confirm your password"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button
            onClick={handleCreate}
            disabled={
              loading ||
              !folderPath ||
              !fileName ||
              !password ||
              !confirmPassword
            }
          >
            {loading ? "Creating..." : "Create Database"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
