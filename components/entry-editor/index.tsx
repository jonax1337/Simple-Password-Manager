"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Save } from "lucide-react";
import { updateEntry, generatePassword } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { emit } from "@tauri-apps/api/event";
import { IconPicker } from "@/components/IconPicker";
import { CustomTitleBar } from "@/components/CustomTitleBar";
import { GeneralTab } from "./GeneralTab";
import { AdvancedTab } from "./AdvancedTab";
import { HistoryTab } from "./HistoryTab";
import { validateUrl } from "./utils/validators";
import type { EntryData } from "@/lib/tauri";

interface EntryEditorProps {
  entry: EntryData;
  onClose: () => void;
  onRefresh: () => void;
  onHasChangesChange?: (hasChanges: boolean) => void;
}

export function EntryEditor({ entry, onClose, onRefresh, onHasChangesChange }: EntryEditorProps) {
  const [formData, setFormData] = useState<EntryData>(entry);
  const [showPassword, setShowPassword] = useState(false);
  const [repeatPassword, setRepeatPassword] = useState("");
  const [hasChanges, setHasChanges] = useState(false);
  const [iconId, setIconId] = useState(0);
  const [urlError, setUrlError] = useState<string | null>(null);
  const [copiedUsername, setCopiedUsername] = useState(false);
  const [copiedPassword, setCopiedPassword] = useState(false);
  const { toast } = useToast();

  // Sync form state when entry changes (e.g., switching entries)
  // Using entry.uuid as dependency since we only want to reset when switching to a different entry
  useEffect(() => {
    setFormData(entry);
    setHasChanges(false);
    setShowPassword(false);
    setRepeatPassword(entry.password); // Auto-fill repeat password
    // Load icon ID from entry.icon_id field
    setIconId(entry.icon_id ?? 0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entry.uuid]);

  // Notify parent when hasChanges state changes
  useEffect(() => {
    onHasChangesChange?.(hasChanges);
  }, [hasChanges, onHasChangesChange]);

  const handleFormChange = (field: keyof EntryData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    setHasChanges(true);
    
    // Validate URL field
    if (field === "url") {
      const { error } = validateUrl(value);
      setUrlError(error);
    }
  };

  const handleIconChange = (newIconId: number) => {
    setIconId(newIconId);
    // Store icon ID in icon_id field
    setFormData((prev) => ({
      ...prev,
      icon_id: newIconId,
    }));
    setHasChanges(true);
  };

  const handleSave = async () => {
    // Validate URL before saving
    const { isValid } = validateUrl(formData.url);
    if (!isValid) {
      toast({
        title: "Invalid URL",
        description: "Please enter a valid URL or leave it empty",
        variant: "destructive",
      });
      return;
    }

    // Check if passwords match - only when both are filled
    if (formData.password && repeatPassword && formData.password !== repeatPassword) {
      toast({
        title: "Passwords don't match",
        description: "Please ensure both password fields match",
        variant: "destructive",
      });
      return;
    }

    // Always ensure we're saving with the correct UUID
    const dataToSave = {
      ...formData,
      uuid: entry.uuid,
    };
    
    try {
      await updateEntry(dataToSave);
      
      // Emit event to main window to refresh and mark as dirty
      await emit('entry-updated', { entryUuid: entry.uuid });
      
      toast({
        title: "Success",
        description: "Entry updated successfully",
        variant: "success",
      });
      setHasChanges(false);
      // Don't close or navigate away - just refresh the data in background
      onRefresh();
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to update entry"),
        variant: "destructive",
      });
    }
  };

  const handleCopy = async (text: string, label: string, field: 'username' | 'password') => {
    try {
      await writeText(text);
      
      // Set copied state for visual feedback
      if (field === 'username') {
        setCopiedUsername(true);
        setTimeout(() => setCopiedUsername(false), 2000);
      } else {
        setCopiedPassword(true);
        setTimeout(() => setCopiedPassword(false), 2000);
      }
      
      toast({
        title: "Copied",
        description: `${label} copied to clipboard`,
        variant: "info",
      });

      setTimeout(async () => {
        await writeText("");
      }, 30000);
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to copy to clipboard",
        variant: "destructive",
      });
    }
  };

  const handleGeneratePassword = async (strength: string) => {
    try {
      let length = 16;
      let useUppercase = true;
      let useLowercase = true;
      let useDigits = true;
      let useSymbols = true;

      switch (strength) {
        case "weak":
          length = 8;
          useSymbols = false;
          break;
        case "medium":
          length = 12;
          break;
        case "strong":
          length = 16;
          break;
        case "very-strong":
          length = 20;
          break;
        case "maximum":
          length = 32;
          break;
      }

      const password = await generatePassword(length, useUppercase, useLowercase, useDigits, useSymbols);
      
      // Auto-fill both password fields
      setFormData((prev) => ({ ...prev, password }));
      setRepeatPassword(password);
      setHasChanges(true);

      toast({
        title: "Password generated",
        description: `Generated ${strength} password (${length} characters)`,
        variant: "success",
      });
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to generate password",
        variant: "destructive",
      });
    }
  };

  return (
    <div className="flex h-full flex-col">
      <CustomTitleBar title={`Edit Entry - ${formData.title}`} hideMaximize />
      {/* Header */}
      <div className="shrink-0 flex items-center gap-3 border-b px-4 py-3 bg-muted/30">
        <IconPicker value={iconId} onChange={handleIconChange} />
        <div>
          <h1 className="text-lg font-semibold">Edit Entry</h1>
          <p className="text-sm text-muted-foreground">You are editing an existing entry.</p>
        </div>
      </div>

      <Tabs defaultValue="general" className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <TabsList className="shrink-0 w-full justify-start rounded-none border-b bg-transparent p-0 h-auto">
          <TabsTrigger value="general" className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2">
            General
          </TabsTrigger>
          <TabsTrigger value="advanced" className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2">
            Advanced
          </TabsTrigger>
          <TabsTrigger value="history" className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2">
            History
          </TabsTrigger>
        </TabsList>

        {/* General Tab */}
        <TabsContent value="general" className="flex-1 m-0 min-h-0 overflow-hidden">
          <GeneralTab
            formData={formData}
            repeatPassword={repeatPassword}
            showPassword={showPassword}
            urlError={urlError}
            copiedUsername={copiedUsername}
            copiedPassword={copiedPassword}
            onFormChange={handleFormChange}
            onRepeatPasswordChange={setRepeatPassword}
            onShowPasswordToggle={() => setShowPassword(!showPassword)}
            onCopy={handleCopy}
            onGeneratePassword={handleGeneratePassword}
            setFormData={setFormData}
            setHasChanges={setHasChanges}
          />
        </TabsContent>

        {/* Advanced Tab */}
        <TabsContent value="advanced" className="flex-1 m-0 min-h-0 overflow-hidden">
          <AdvancedTab
            formData={formData}
            setFormData={setFormData}
            setHasChanges={setHasChanges}
          />
        </TabsContent>

        {/* History Tab */}
        <TabsContent value="history" className="flex-1 m-0 min-h-0 overflow-hidden">
          <HistoryTab
            entry={entry}
            formData={formData}
            setFormData={setFormData}
            setRepeatPassword={setRepeatPassword}
            setHasChanges={setHasChanges}
          />
        </TabsContent>
      </Tabs>

      {/* Footer */}
      <div className="shrink-0 flex items-center justify-end gap-2 border-t px-4 py-3 bg-background">
        <Button 
          variant="outline" 
          onClick={onClose}
        >
          Cancel
        </Button>
        <Button 
          onClick={handleSave} 
          disabled={!hasChanges || !!urlError || !!(formData.password && repeatPassword && formData.password !== repeatPassword)}
        >
          <Save className="mr-2 h-4 w-4" />
          Save
        </Button>
      </div>
    </div>
  );
}
