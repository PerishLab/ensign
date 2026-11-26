{{- define "ensign.storeseat" -}}
{{- $pg := .Values.postgres | default dict -}}
{{- $urlSecret := $pg.urlSecret | default dict -}}
{{- if $urlSecret.name -}}
{{ $urlSecret.name }}
{{- else -}}
{{ .Release.Name }}-store
{{- end -}}
{{- end -}}

{{- define "ensign.storekey" -}}
{{- $pg := .Values.postgres | default dict -}}
{{- $urlSecret := $pg.urlSecret | default dict -}}
{{- if $urlSecret.name -}}
{{ $urlSecret.key }}
{{- else -}}
ENSIGN_STORE_URL
{{- end -}}
{{- end -}}

{{- define "ensign.pgpass" -}}
{{- $pg := .Values.postgres | default dict -}}
{{- $held := lookup "v1" "Secret" .Release.Namespace (printf "%s-store" .Release.Name) -}}
{{- if and $held $held.data.PGPASSWORD -}}
{{ $held.data.PGPASSWORD | b64dec }}
{{- else if $pg.password -}}
{{ $pg.password }}
{{- else -}}
{{ randAlphaNum 32 }}
{{- end -}}
{{- end -}}


