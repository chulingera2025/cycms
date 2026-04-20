export function LoadingSpinner({ text = '加载中...' }: { text?: string }) {
  return (
    <div className="loading-spinner">
      <div className="spinner" />
      <span>{text}</span>
    </div>
  );
}
