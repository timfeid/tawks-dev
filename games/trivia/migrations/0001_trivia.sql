-- Idempotent so it can baseline a database that already has these tables.

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$;

CREATE TABLE IF NOT EXISTS trivia (
    id character varying PRIMARY KEY,
    community_id text NOT NULL,
    title text NOT NULL,
    description text NOT NULL,
    closes_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    topic character varying(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS trivia_questions (
    id character varying PRIMARY KEY,
    trivia_id character varying NOT NULL REFERENCES trivia(id) ON DELETE CASCADE,
    "order" integer NOT NULL CONSTRAINT trivia_questions_order_nonneg CHECK ("order" >= 0),
    text text NOT NULL,
    annotation_url text,
    random_fact text,
    incorrect_fluff text,
    max_points integer NOT NULL CONSTRAINT trivia_questions_max_points_min CHECK (max_points >= 0),
    duration_seconds integer DEFAULT 15 NOT NULL CONSTRAINT trivia_questions_duration_min CHECK (duration_seconds > 0),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    audio_url text
);

CREATE TABLE IF NOT EXISTS trivia_answers (
    id character varying PRIMARY KEY,
    trivia_question_id character varying NOT NULL REFERENCES trivia_questions(id) ON DELETE CASCADE,
    text text NOT NULL,
    is_correct boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    "order" integer NOT NULL,
    audio_url text
);

CREATE TABLE IF NOT EXISTS trivia_attempts (
    id character varying PRIMARY KEY,
    trivia_id character varying NOT NULL REFERENCES trivia(id) ON DELETE CASCADE,
    user_id text NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    finished_at timestamp with time zone,
    final_score integer DEFAULT 0 NOT NULL CONSTRAINT trivia_attempts_score_nonneg CHECK (final_score >= 0)
);

CREATE TABLE IF NOT EXISTS trivia_user_answers (
    id character varying PRIMARY KEY,
    attempt_id character varying NOT NULL REFERENCES trivia_attempts(id) ON DELETE CASCADE,
    trivia_question_id character varying NOT NULL REFERENCES trivia_questions(id) ON DELETE CASCADE,
    trivia_answer_id character varying REFERENCES trivia_answers(id) ON DELETE SET NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    finished_at timestamp with time zone,
    points_awarded integer CONSTRAINT trivia_user_answers_points_nonneg CHECK (points_awarded IS NULL OR points_awarded >= 0)
);

CREATE INDEX IF NOT EXISTS trivia_by_closes_at ON trivia (closes_at);
CREATE INDEX IF NOT EXISTS trivia_by_community_id ON trivia (community_id);
CREATE INDEX IF NOT EXISTS trivia_questions_by_trivia ON trivia_questions (trivia_id);
CREATE UNIQUE INDEX IF NOT EXISTS trivia_questions_unique_order ON trivia_questions (trivia_id, "order");
CREATE INDEX IF NOT EXISTS trivia_answers_by_question ON trivia_answers (trivia_question_id);
CREATE UNIQUE INDEX IF NOT EXISTS trivia_answers_one_correct ON trivia_answers (trivia_question_id) WHERE is_correct;
CREATE INDEX IF NOT EXISTS trivia_attempts_by_finished_at ON trivia_attempts (finished_at);
CREATE INDEX IF NOT EXISTS trivia_attempts_by_trivia ON trivia_attempts (trivia_id);
CREATE INDEX IF NOT EXISTS trivia_attempts_by_user ON trivia_attempts (user_id);
CREATE UNIQUE INDEX IF NOT EXISTS trivia_attempts_one_per_user ON trivia_attempts (trivia_id, user_id);
CREATE INDEX IF NOT EXISTS trivia_user_answers_by_attempt ON trivia_user_answers (attempt_id);
CREATE INDEX IF NOT EXISTS trivia_user_answers_by_question ON trivia_user_answers (trivia_question_id);
CREATE UNIQUE INDEX IF NOT EXISTS trivia_user_answers_one_per_question ON trivia_user_answers (attempt_id, trivia_question_id);

CREATE OR REPLACE TRIGGER trg_trivia_updated_at BEFORE UPDATE ON trivia
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE OR REPLACE TRIGGER trg_trivia_questions_updated_at BEFORE UPDATE ON trivia_questions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
