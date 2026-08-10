CREATE TABLE character_widgets (
    character_id  BIGINT   NOT NULL REFERENCES characters(id)   ON DELETE CASCADE,
    widget_id     BIGINT   NOT NULL REFERENCES user_widgets(id) ON DELETE CASCADE,
    position      INTEGER  NOT NULL,

    PRIMARY KEY (character_id, widget_id)
);

CREATE INDEX character_widgets_character_id_idx
    ON character_widgets (character_id, position);
