slint::slint! {
    component Border {
        in property <brush> background-color;
        in property <brush> border-color;
        Rectangle {
            background: parent.border-color;
        }
        Rectangle {
            width: (parent.width) - 2px;
            height: (parent.height) - 2px;
            background: parent.background-color;
        }
    }
    export component App inherits Window {
        background: rgb(13, 17, 23);
        no-frame: true;
        title: "Hyperion";
        width: 990px;
        height: 450px;

        Border {
            width: parent.width;
            height: parent.height;
            background-color: rgb(13, 17, 23);
            border-color: rgb(61, 68, 77);
        }

        Text {
            text: "Exam Controller";
            font-size: 32px;
            y: 10px;
        }
    }
}