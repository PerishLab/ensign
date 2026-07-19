{{- define "ensign.pgurl" -}}
{{- if .Values.postgres.url -}}
{{ .Values.postgres.url }}
{{- else -}}
host={{ .Release.Name }}-pg port=5432 user={{ .Values.postgres.user }} password={{ .Values.postgres.password }} dbname={{ .Values.postgres.db }}
{{- end -}}
{{- end -}}
