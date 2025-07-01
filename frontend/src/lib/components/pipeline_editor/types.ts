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

export interface RegexPatternConfig {
  id: string;
  name: string;
  regex: string;
  captureGroupIndex?: number;
}

export interface Pipeline {
  id?: string;
  name: string;
  org_id: string;
  stages: Stage[];
}

export interface EditorPromptTemplate {
  name: string;
  text: string;
}
