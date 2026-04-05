export default function WorkflowEmptyState({ description, title }) {
  return (
    <main className="screen">
      <section className="empty-state">
        <div className="eyebrow">dbgflow</div>
        <h1>{title}</h1>
        <p>{description}</p>
      </section>
    </main>
  );
}
