{{- define "hipcortex.fullname" -}}
{{- .Release.Name }}-hipcortex
{{- end }}

{{- define "hipcortex.labels" -}}
app.kubernetes.io/name: hipcortex
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion }}
{{- end }}

{{- define "hipcortex.selectorLabels" -}}
app.kubernetes.io/name: hipcortex
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}
