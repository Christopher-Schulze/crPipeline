export interface Stage {
  id: string;
  type: string;
  command?: string | null;
  prompt_name?: string | null;
  ocr_engine?: 'default' | 'external';
  ocr_stage_endpoint?: string | null;
  ocr_stage_key?: string | null;
  config?: {
    strategy?: string;
    parameters?: any;
    template?: string;
    summaryFields?: string[] | null;
    _summaryFieldsString?: string;
  } | null;
}

export interface Pipeline {
  id?: string;
  org_id: string;
  name: string;
  stages: Stage[];
}

export interface Document {
  id: string;
  filename: string;
  display_name: string;
  is_target: boolean;
  upload_date: string;
  pages?: number;
  expires_at?: string | null;
}

export interface OrgSettings {
  org_id: string;
  monthly_upload_quota: number;
  monthly_analysis_quota: number;
  accent_color: string;
  ai_api_endpoint?: string | null;
  ai_api_key?: string | null;
  ocr_api_endpoint?: string | null;
  ocr_api_key?: string | null;
  prompt_templates?: { id?: string; name: string; text: string }[] | null;
  ai_custom_headers?: { id: string; name: string; value: string }[] | null;
}

export interface AuditLog {
  id: string;
  org_id: string;
  user_id: string;
  action: string;
  created_at: string;
}
