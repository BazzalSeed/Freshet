import type { ParsedDoc } from "../../lib/parseDoc";
import { Cited } from "./Cited";
import { MyNotes } from "./MyNotes";
import "./Document.css";

/**
 * The reading view's document column. Renders the living markdown document
 * top-to-bottom: title, metadata, and the four movements (What changed,
 * Current understanding, Open questions, My notes). Each movement label gets a
 * stable `id` slug so the Outline can jump to it.
 */
export function Document({
  doc,
  onSaveNotes,
}: {
  doc: ParsedDoc;
  onSaveNotes: (block: string) => void;
}) {
  return (
    <article className="document">
      <h1 className="document-title">{doc.title}</h1>
      <p className="document-updated">{doc.updatedLabel}</p>

      <section className="document-section" aria-labelledby="sec-what-changed">
        <h2 id="sec-what-changed" className="section-label" data-accent>
          What changed
        </h2>
        <ul className="document-bullets">
          {doc.whatChanged.map((bullet, i) => (
            <li key={i}>
              <Cited text={bullet} sources={doc.sources} />
            </li>
          ))}
        </ul>
      </section>

      <section className="document-section" aria-labelledby="sec-current-understanding">
        <h2 id="sec-current-understanding" className="section-label">
          Current understanding
        </h2>
        {doc.current.map((sec, i) => (
          <div className="document-subsection" key={i}>
            {sec.heading ? <h3 className="subsection-heading">{sec.heading}</h3> : null}
            {sec.body.map((para, j) => (
              <p className="document-paragraph" key={j}>
                <Cited text={para} sources={doc.sources} />
              </p>
            ))}
          </div>
        ))}
      </section>

      <section className="document-section" aria-labelledby="sec-open-questions">
        <h2 id="sec-open-questions" className="section-label">
          Open questions
        </h2>
        <ul className="document-bullets">
          {doc.openQuestions.map((bullet, i) => (
            <li key={i}>
              <Cited text={bullet} sources={doc.sources} />
            </li>
          ))}
        </ul>
      </section>

      {/*
        No aria-labelledby here: the editable textarea below already carries
        aria-label="My notes" as the meaningful control label. Labelling the
        section with the same text would expose two "My notes" accessible
        elements (section + control), so we let the heading stand on its own.
      */}
      <section className="document-section">
        <h2 id="sec-my-notes" className="section-label">
          My notes
        </h2>
        <MyNotes markdown={doc.myNotes} onSave={onSaveNotes} />
      </section>
    </article>
  );
}
