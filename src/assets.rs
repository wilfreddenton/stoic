pub const CSS_STR: &str = r#"
*,
*::before,
*::after {
  box-sizing: border-box;
}

#container {
  position: relative;
  max-width: 500px;
  margin: 0 auto;
}

#posts-list {
  list-style-type: none;
}

#posts-list .posts-list-item a {
  display: flex;
}

#posts-list .posts-list-item .posts-list-item-title {
  flex-grow: 8;
}

#posts-list .posts-list-item .posts-list-item-time {
  flex-grow: 4;
  text-align: right;
}

@media (max-width: 576px) {
  #posts-list .posts-list-item a {
    flex-direction: column;
  }

  #posts-list .posts-list-item .posts-list-item-time {
    text-align: left;
  }
}
"#;

pub const JS_STR: &str = r#"
console.log("Hello, World!")
"#;
