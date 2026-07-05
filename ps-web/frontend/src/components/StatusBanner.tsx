interface Props {
  kind: 'error' | 'hint'
  title: string
  body: string
}

export default function StatusBanner({ kind, title, body }: Props) {
  const tone =
    kind === 'error'
      ? 'border-red-400/40 bg-red-500/10 text-red-200'
      : 'border-amber-300/30 bg-amber-400/10 text-amber-100'
  return (
    <div className={`rounded-xl border px-4 py-3 ${tone}`} role="alert">
      <h3 className="text-sm font-semibold">{title}</h3>
      <p className="mt-1 text-sm opacity-90">{body}</p>
    </div>
  )
}
