import { onEscape } from '@/lib/keys';

export function ConfirmSheet(props: { title: string; body: string; action: string; onConfirm: () => Promise<void>; onDone: () => void }) {
  return (
    <div className="sheet" role="alertdialog" aria-labelledby="confirm-title" onKeyDown={onEscape(props.onDone)}>
      <h1 id="confirm-title">{props.title}</h1>
      <p className="lede">{props.body}</p>
      <div className="actions">
        <button type="button" className="btn ghost" autoFocus onClick={props.onDone}>
          Cancel
        </button>
        <button type="button" className="btn danger" onClick={() => props.onConfirm().then(props.onDone)}>
          {props.action}
        </button>
      </div>
    </div>
  );
}
