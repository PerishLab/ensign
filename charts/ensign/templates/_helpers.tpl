{{- define "ensign.storeseat" -}}
{{- if .Values.postgres.urlSecret.name -}}
{{ .Values.postgres.urlSecret.name }}
{{- else -}}
{{ .Release.Name }}-store
{{- end -}}
{{- end -}}

{{- define "ensign.storekey" -}}
{{- if .Values.postgres.urlSecret.name -}}
{{ .Values.postgres.urlSecret.key }}
{{- else -}}
API_STORE_URL
{{- end -}}
{{- end -}}

{{- define "ensign.pgpass" -}}
{{- $held := lookup "v1" "Secret" .Release.Namespace (printf "%s-store" .Release.Name) -}}
{{- if and $held $held.data.PGPASSWORD -}}
{{ $held.data.PGPASSWORD | b64dec }}
{{- else if .Values.postgres.password -}}
{{ .Values.postgres.password }}
{{- else -}}
{{ randAlphaNum 32 }}
{{- end -}}
{{- end -}}

{{- define "ensign.pgurl" -}}
{{- if .Values.postgres.url -}}
{{ .Values.postgres.url }}
{{- else -}}
host={{ .Release.Name }}-pg port=5432 user={{ .Values.postgres.user }} password={{ include "ensign.pgpass" . }} dbname={{ .Values.postgres.db }}
{{- end -}}
{{- end -}}
