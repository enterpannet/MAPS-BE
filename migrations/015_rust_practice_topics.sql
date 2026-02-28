CREATE TABLE rust_practice_topics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    s1_title TEXT NOT NULL,
    s1_content TEXT NOT NULL,
    s1_code TEXT,
    s2_title TEXT NOT NULL,
    s2_description TEXT NOT NULL,
    s2_code TEXT NOT NULL,
    s3_title TEXT NOT NULL,
    s3_description TEXT NOT NULL,
    s3_code_with_blanks TEXT NOT NULL,
    s3_solution TEXT NOT NULL,
    s4_title TEXT NOT NULL,
    s4_task TEXT NOT NULL,
    s4_hint TEXT,
    s4_solution TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
